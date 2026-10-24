//! Source-bound read-only comparisons and atomic device → project adoption.
//!
//! No deserializable evidence or value-accepting constructor is exposed. Retain
//! these transient objects in the caller; request payloads may select row indices
//! only. See `docs/contracts/alpha16-comparison-adoption.md` for trust/locking rules.
use crate::device_download::prepare_device_download;
use knx_core::command::comparison_adoption::{
    AdoptionCommandError, LinkAdoption, ParameterAdoption, PreparedComparisonAdoption,
};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::{
    ComObjectInstanceId, CommandStack, ContactableAddress, DeviceId, Direction, GroupAddress,
    GroupAddressId, IndividualAddress, Project,
};
use knx_net::commissioning::memory_download::{compare_with_plan, CompareError, PlanComparison};
use knx_net::commissioning::SessionTiming;
use knx_net::management::ManagementTransport;
use knx_productdb::image::{DownloadImage, ImageRequest, ParameterAttribution};
use knx_productdb::image_comparison::{decode, AssociationSlot, ComparisonRow};
use knx_productdb::Connection;
use knx_store::project_history::NativeSnapshot;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Caller-owned current session identity, never authority supplied by a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonScope {
    pub server_incarnation: String,
    pub project_incarnation: u64,
}
#[derive(Debug)]
pub enum AdoptionError {
    InvalidScope,
    Stale,
    SourceChanged,
    Preparation(String),
    Compare(CompareError),
    EmptySelection,
    DuplicateRow(usize),
    MissingRow(usize),
    UnsupportedRow {
        index: usize,
        kind: &'static str,
    },
    UnknownRow {
        index: usize,
    },
    AmbiguousParameter(String),
    CandidateFieldChanged(String),
    /// An association row (selected or not) is unknown on one side, so the
    /// link set an adoption would replace is not completely known.
    IncompleteAssociationTable {
        index: usize,
    },
    /// A differing association row names an adopted object but is unselected.
    IncompleteObjectSelection {
        object: u8,
        index: usize,
    },
    /// No project group address has this raw address; adoption creates none.
    GroupAddressNotInProject {
        index: usize,
        group_address: GroupAddress,
    },
    /// Several project group addresses have this raw address.
    AmbiguousGroupAddress {
        index: usize,
        group_address: GroupAddress,
    },
    /// No project object of the device with this number is active.
    ObjectNotActive {
        index: usize,
        object: u8,
    },
    /// Several, module-based, or differently linked instances share the number.
    AmbiguousObject {
        index: usize,
        object: u8,
    },
    /// The device order needs a sending flag the table cannot prove.
    SendingLinkUndetermined {
        object: u8,
    },
    /// The rebuilt candidate does not hold the device's links for the object.
    CandidateLinksChanged {
        object: u8,
    },
    Command(AdoptionCommandError),
    Native(String),
}
impl std::fmt::Display for AdoptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidScope => f.write_str("invalid comparison session scope"),
            Self::Stale => f.write_str(
                "comparison project, scope, revision or complete history changed; compare again",
            ),
            Self::SourceChanged => {
                f.write_str("comparison source request/image/plan changed; compare again")
            }
            Self::Preparation(error) => write!(f, "comparison preparation: {error}"),
            Self::Compare(error) => write!(f, "comparison read: {error}"),
            Self::EmptySelection => f.write_str("select at least one supported comparison row"),
            Self::DuplicateRow(index) => write!(f, "duplicate requested comparison row {index}"),
            Self::MissingRow(index) => write!(f, "comparison row {index} does not exist"),
            Self::UnsupportedRow { index, kind } => write!(
                f,
                "comparison row {index} ({kind}) has no supported project adoption mapping"
            ),
            Self::UnknownRow { index } => {
                write!(f, "comparison row {index} is not completely decoded")
            }
            Self::AmbiguousParameter(reference) => {
                write!(f, "ambiguous parameter attribution: {reference}")
            }
            Self::CandidateFieldChanged(reference) => write!(
                f,
                "adopted field {reference} no longer has the same verified placement/value"
            ),
            Self::IncompleteAssociationTable { index } => write!(
                f,
                "association row {index} is not completely decoded, so the device's links are not \
                 completely known"
            ),
            Self::IncompleteObjectSelection { object, index } => write!(
                f,
                "association row {index} also differs for object {object}; select every \
                 differing association row of that object"
            ),
            Self::GroupAddressNotInProject {
                index,
                group_address,
            } => write!(
                f,
                "association row {index}: group address {} is not in the project; adoption \
                 creates none",
                ga_text(*group_address)
            ),
            Self::AmbiguousGroupAddress {
                index,
                group_address,
            } => write!(
                f,
                "association row {index}: group address {} exists more than once in the project",
                ga_text(*group_address)
            ),
            Self::ObjectNotActive { index, object } => write!(
                f,
                "association row {index}: no active communication object {object} of this device \
                 in the project"
            ),
            Self::AmbiguousObject { index, object } => write!(
                f,
                "association row {index}: communication object {object} is ambiguous in the \
                 project (several, module-based or differently linked instances)"
            ),
            Self::SendingLinkUndetermined { object } => write!(
                f,
                "object {object}: the device's link order needs a sending link the association \
                 table cannot prove; existing sending flags are not guessed"
            ),
            Self::CandidateLinksChanged { object } => write!(
                f,
                "object {object}: the rebuilt project links do not equal the device's"
            ),
            Self::Command(error) => write!(f, "{error}"),
            Self::Native(error) => write!(
                f,
                "comparison adoption native/history preflight or persistence: {error}"
            ),
        }
    }
}
impl std::error::Error for AdoptionError {}
impl From<AdoptionCommandError> for AdoptionError {
    fn from(value: AdoptionCommandError) -> Self {
        Self::Command(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceAuthority {
    request: ImageRequest,
    image: DownloadImage,
    plan: MemoryDownloadPlan,
}
/// Construct all source-derived inputs under one SQLite read snapshot. Refuse
/// an already active transaction rather than changing its ownership/semantics.
fn source(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
) -> Result<SourceAuthority, AdoptionError> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    let prepared = prepare_device_download(&transaction, project, target)
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    transaction
        .commit()
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    Ok(SourceAuthority {
        request: prepared.request,
        image: prepared.image,
        plan: prepared.plan,
    })
}

/// Prepared from the actual project/source, not caller-supplied image/plan bytes.
#[derive(Debug)]
pub struct PreparedComparison {
    project: Project,
    scope: ComparisonScope,
    device: DeviceId,
    revision: u64,
    target: ContactableAddress,
    source: SourceAuthority,
}
impl PreparedComparison {
    pub fn device(&self) -> DeviceId {
        self.device
    }
    pub fn target(&self) -> IndividualAddress {
        self.target.address()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn scope(&self) -> &ComparisonScope {
        &self.scope
    }
    /// Display only. Returning shared references cannot change read authority.
    pub fn image(&self) -> &DownloadImage {
        &self.source.image
    }
    pub fn plan(&self) -> &MemoryDownloadPlan {
        &self.source.plan
    }
    pub fn program_id(&self) -> &str {
        &self.source.request.program_id
    }
    /// Call immediately before tunnel admission, under the project/stack locks.
    /// Recheck after the read via `ObservedComparison::is_current`.
    pub fn is_current(
        &self,
        project: &Project,
        stack: &CommandStack,
        scope: &ComparisonScope,
    ) -> bool {
        use knx_core::commissioning::pending::programming_content;
        let devices = BTreeSet::from([self.device]);
        self.scope == *scope
            && stack.comparison_revision(self.device) == self.revision
            && programming_content(&self.project, &devices)
                == programming_content(project, &devices)
    }
    /// Runs the existing read-only identity-checked comparison itself. No public
    /// API can promote a fabricated `PlanComparison` or JSON result to evidence.
    pub async fn observe<T: ManagementTransport>(
        self,
        transport: &T,
        timing: SessionTiming,
    ) -> Result<ObservedComparison, AdoptionError> {
        let comparison = compare_with_plan(transport, self.target, timing, &self.source.plan)
            .await
            .map_err(AdoptionError::Compare)?;
        let rows = decode(&self.source.image, &comparison.held, &self.source.plan);
        Ok(ObservedComparison {
            prepared: self,
            comparison,
            rows,
        })
    }
}

pub fn prepare_comparison(
    conn: &Connection,
    project: &Project,
    stack: &CommandStack,
    scope: &ComparisonScope,
    device: DeviceId,
) -> Result<PreparedComparison, AdoptionError> {
    if scope.server_incarnation.is_empty() || scope.server_incarnation.len() > 128 {
        return Err(AdoptionError::InvalidScope);
    }
    let revision = stack.comparison_revision(device);
    if revision == u64::MAX {
        return Err(AdoptionError::Stale);
    }
    let target = project
        .devices
        .get(device)
        .and_then(|d| d.address)
        .ok_or_else(|| {
            AdoptionError::Preparation("device has no contactable project address".into())
        })?;
    // Uses existing single-device-at-address admission, not first-match selection.
    let authority = source(conn, project, target)?;
    let target =
        ContactableAddress::new(target).map_err(|e| AdoptionError::Preparation(e.to_string()))?;
    Ok(PreparedComparison {
        project: project.clone(),
        scope: scope.clone(),
        device,
        revision,
        target,
        source: authority,
    })
}

/// Genuine successful read; no public constructor, mutable getters or serde.
#[derive(Debug)]
pub struct ObservedComparison {
    prepared: PreparedComparison,
    comparison: PlanComparison,
    rows: Vec<ComparisonRow>,
}
impl ObservedComparison {
    pub fn rows(&self) -> &[ComparisonRow] {
        &self.rows
    }
    pub fn comparison(&self) -> &PlanComparison {
        &self.comparison
    }
    pub fn preparation(&self) -> &PreparedComparison {
        &self.prepared
    }
    pub fn is_current(
        &self,
        project: &Project,
        stack: &CommandStack,
        scope: &ComparisonScope,
    ) -> bool {
        self.prepared.is_current(project, stack, scope)
    }
}

/// Complete preview, including unsupported/unread rows. Candidate authority and
/// Core command are immutable and are never reconstructed from a client response.
#[derive(Debug)]
pub struct PreparedAdoption {
    before: NativeSnapshot,
    scope: ComparisonScope,
    target: IndividualAddress,
    original_source: SourceAuthority,
    candidate_source: SourceAuthority,
    command: PreparedComparisonAdoption,
    rows: Vec<ComparisonRow>,
    selected: Vec<usize>,
    link_changes: Vec<LinkChange>,
}
impl PreparedAdoption {
    pub fn rows(&self) -> &[ComparisonRow] {
        &self.rows
    }
    pub fn selected(&self) -> &[usize] {
        &self.selected
    }
    pub fn changed(&self) -> bool {
        self.command.changed()
    }
    pub fn candidate(&self) -> (&Project, &CommandStack) {
        self.command.candidate()
    }
    /// Every link the edit adds or removes, removals first.
    pub fn link_changes(&self) -> &[LinkChange] {
        &self.link_changes
    }
}

/// One object ↔ group-address link the adoption adds or removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkChange {
    pub add: bool,
    /// The group object number (ASAP).
    pub object: u8,
    pub com_object: ComObjectInstanceId,
    pub group_address: GroupAddress,
    pub group_address_id: GroupAddressId,
    pub sending: bool,
}

fn ga_text(address: GroupAddress) -> String {
    let raw = address.raw();
    format!("{}/{}/{}", raw >> 11, (raw >> 8) & 7, raw & 255)
}

/// Presentation hint for one decoded row; `preview_adoption` decides.
pub fn adoption_hint(row: &ComparisonRow) -> bool {
    match row.kind {
        "parameter" => {
            row.parameter.is_some()
                && row.reason.is_none()
                && row.project.is_some()
                && row.device.is_some()
        }
        "association" => row
            .association
            .is_some_and(|a| a.project.is_some() && a.device.is_some() && a.project != a.device),
        _ => false,
    }
}

fn entry_object(slot: Option<AssociationSlot>) -> Option<u8> {
    match slot? {
        AssociationSlot::Entry(entry) => Some(entry.object),
        AssociationSlot::Beyond => None,
    }
}

#[derive(Default)]
struct LinkPlan {
    edits: Vec<LinkAdoption>,
    changes: Vec<LinkChange>,
    /// Touched object → the device's group addresses for it, in table order.
    device: BTreeMap<u8, Vec<GroupAddress>>,
}

/// Derives link edits for the selected association rows; see the
/// association amendment of `docs/contracts/alpha16-comparison-adoption.md`.
fn plan_links(
    project: &Project,
    device: DeviceId,
    program_id: &str,
    active: &BTreeSet<String>,
    rows: &[ComparisonRow],
    selected: &[usize],
) -> Result<LinkPlan, AdoptionError> {
    let mut touched = BTreeSet::new();
    for &index in selected {
        let attribution = rows[index]
            .association
            .filter(|a| a.project.is_some() && a.device.is_some())
            .ok_or(AdoptionError::UnknownRow { index })?;
        touched.extend(entry_object(attribution.project));
        touched.extend(entry_object(attribution.device));
    }
    let mut plan = LinkPlan::default();
    let mut first_row: BTreeMap<u8, usize> = BTreeMap::new();
    let mut device_rows: BTreeMap<u8, Vec<(GroupAddress, usize)>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        match row.kind {
            "associationCount" if row.device.is_none() || row.project.is_none() => {
                return Err(AdoptionError::IncompleteAssociationTable { index });
            }
            "association" => {}
            _ => continue,
        }
        let Some(attribution) = row.association else {
            continue;
        };
        let (Some(project_slot), Some(device_slot)) = (attribution.project, attribution.device)
        else {
            return Err(AdoptionError::IncompleteAssociationTable { index });
        };
        let named: Vec<u8> = [
            entry_object(Some(project_slot)),
            entry_object(Some(device_slot)),
        ]
        .into_iter()
        .flatten()
        .filter(|object| touched.contains(object))
        .collect();
        for &object in &named {
            first_row.entry(object).or_insert(index);
        }
        if project_slot != device_slot && !selected.contains(&index) {
            if let Some(&object) = named.first() {
                return Err(AdoptionError::IncompleteObjectSelection { object, index });
            }
        }
        if let AssociationSlot::Entry(entry) = device_slot {
            if touched.contains(&entry.object) {
                device_rows
                    .entry(entry.object)
                    .or_default()
                    .push((entry.group_address, index));
            }
        }
    }
    let addresses: Vec<(GroupAddressId, GroupAddress)> = project
        .installations
        .iter()
        .flat_map(|i| &i.group_addresses)
        .map(|entry| (entry.id, entry.address))
        .collect();
    let instance = project.devices.get(device).ok_or_else(|| {
        AdoptionError::Preparation("compared device is not in the project".into())
    })?;
    for &object in &touched {
        let index = first_row.get(&object).copied().unwrap_or(selected[0]);
        let wanted = device_rows.remove(&object).unwrap_or_default();
        let same: Vec<_> = instance
            .com_objects
            .iter()
            .filter_map(|id| project.devices.com_object(*id))
            .filter(|o| o.number == u16::from(object))
            .collect();
        if same.iter().any(|o| o.module_instance.is_some()) {
            return Err(AdoptionError::AmbiguousObject { index, object });
        }
        let holders: Vec<&knx_core::ComObjectInstance> = same
            .iter()
            .copied()
            .filter(|o| !o.links.is_empty())
            .collect();
        let chosen = if wanted.is_empty() {
            match holders.as_slice() {
                [] => {
                    plan.device.insert(object, Vec::new());
                    continue;
                }
                [holder] => *holder,
                _ => return Err(AdoptionError::AmbiguousObject { index, object }),
            }
        } else {
            let live: Vec<&knx_core::ComObjectInstance> = same
                .iter()
                .copied()
                .filter(|o| {
                    active.contains(&knx_productdb::enrich::com_object_lookup_id(
                        program_id,
                        &o.source.ets_id,
                        false,
                    ))
                })
                .collect();
            let chosen = match live.as_slice() {
                [] => return Err(AdoptionError::ObjectNotActive { index, object }),
                [one] => *one,
                _ => return Err(AdoptionError::AmbiguousObject { index, object }),
            };
            if holders.iter().any(|holder| holder.id != chosen.id) {
                return Err(AdoptionError::AmbiguousObject { index, object });
            }
            chosen
        };
        let wanted_addresses: Vec<GroupAddress> = wanted.iter().map(|(ga, _)| *ga).collect();
        let mut resolved_addresses = BTreeMap::new();
        for &(group_address, row) in &wanted {
            if resolved_addresses.contains_key(&group_address) {
                continue;
            }
            let ids: Vec<GroupAddressId> = addresses
                .iter()
                .filter(|(_, address)| *address == group_address)
                .map(|(id, _)| *id)
                .collect();
            let ga = match ids.as_slice() {
                [] => {
                    return Err(AdoptionError::GroupAddressNotInProject {
                        index: row,
                        group_address,
                    })
                }
                [one] => *one,
                _ => {
                    return Err(AdoptionError::AmbiguousGroupAddress {
                        index: row,
                        group_address,
                    })
                }
            };
            resolved_addresses.insert(group_address, ga);
        }
        let mut retained = Vec::new();
        for link in &chosen.links {
            let address = addresses
                .iter()
                .find(|(id, _)| *id == link.ga)
                .map(|(_, address)| *address)
                .ok_or(AdoptionError::AmbiguousObject { index, object })?;
            if wanted_addresses.contains(&address) {
                retained.push(address);
                continue;
            }
            plan.edits.push(LinkAdoption {
                add: false,
                com_object: chosen.id,
                ga: link.ga,
                direction: link.direction,
            });
            plan.changes.push(LinkChange {
                add: false,
                object,
                com_object: chosen.id,
                group_address: address,
                group_address_id: link.ga,
                sending: link.direction == Direction::Send,
            });
        }
        let fresh = retained.is_empty();
        let mut added = BTreeSet::new();
        for (position, &(group_address, _)) in wanted.iter().enumerate() {
            if retained.contains(&group_address) || !added.insert(group_address) {
                continue;
            }
            let ga = resolved_addresses[&group_address];
            // Only an object without retained links takes the device's first
            // entry as its sending link; retained flags are never changed.
            let sending = fresh && position == 0;
            let direction = if sending {
                Direction::Send
            } else {
                Direction::Receive
            };
            plan.edits.push(LinkAdoption {
                add: true,
                com_object: chosen.id,
                ga,
                direction,
            });
            plan.changes.push(LinkChange {
                add: true,
                object,
                com_object: chosen.id,
                group_address,
                group_address_id: ga,
                sending,
            });
        }
        plan.device.insert(object, wanted_addresses);
    }
    // Removals first, as the command applies them.
    plan.changes.sort_by_key(|change| change.add);
    Ok(plan)
}

/// Every touched object's rebuilt entries, in table order, equal the device's.
fn check_candidate_links(
    plan: &LinkPlan,
    candidate_rows: &[ComparisonRow],
) -> Result<(), AdoptionError> {
    let mut rebuilt: BTreeMap<u8, Vec<GroupAddress>> = BTreeMap::new();
    for row in candidate_rows.iter().filter(|r| r.kind == "association") {
        match row.association.and_then(|a| a.project) {
            Some(AssociationSlot::Entry(entry)) => {
                rebuilt
                    .entry(entry.object)
                    .or_default()
                    .push(entry.group_address);
            }
            Some(AssociationSlot::Beyond) => {}
            None => {
                let object = plan.device.keys().next().copied().unwrap_or_default();
                return Err(AdoptionError::CandidateLinksChanged { object });
            }
        }
    }
    for (object, wanted) in &plan.device {
        let got = rebuilt.remove(object).unwrap_or_default();
        if got == *wanted {
            continue;
        }
        let sorted = |list: &[GroupAddress]| {
            let mut list = list.to_vec();
            list.sort_unstable();
            list
        };
        return Err(if sorted(&got) == sorted(wanted) {
            AdoptionError::SendingLinkUndetermined { object: *object }
        } else {
            AdoptionError::CandidateLinksChanged { object: *object }
        });
    }
    Ok(())
}

fn same_placement(a: &ParameterAttribution, b: &ParameterAttribution) -> bool {
    a.reference == b.reference && a.segment == b.segment && a.field == b.field
}
fn ambiguous(image: &DownloadImage, selected: &ParameterAttribution) -> bool {
    let start_bit = |p: &ParameterAttribution| {
        image.segment(&p.segment).map(|s| {
            (u64::from(s.address) + u64::from(p.field.offset)) * 8 + u64::from(p.field.bit_offset)
        })
    };
    let Some(start) = start_bit(selected) else {
        return true;
    };
    let end = start + u64::from(selected.field.size_in_bit);
    image
        .parameter_fields
        .iter()
        .filter(|p| p.reference == selected.reference)
        .count()
        != 1
        || image
            .parameter_fields
            .iter()
            .filter(|p| p.reference != selected.reference)
            .any(|p| {
                start_bit(p).is_some_and(|other| {
                    start < other + u64::from(p.field.size_in_bit) && other < end
                })
            })
}

/// Preflight the complete native candidate plus both history directions using
/// the actual native history encoder/admission, preserving retained context.
fn preflight(snapshot: &NativeSnapshot, stack: &CommandStack) -> Result<(), AdoptionError> {
    let memory = knx_store::open_and_migrate_in_memory()
        .map_err(|e| AdoptionError::Native(e.to_string()))?;
    knx_store::project_history::save_editor(&memory, snapshot, stack, None, true)
        .map_err(|e| AdoptionError::Native(e.to_string()))?;
    Ok(())
}

pub fn preview_adoption(
    conn: &Connection,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    scope: &ComparisonScope,
    observed: &ObservedComparison,
    selected: &[usize],
) -> Result<PreparedAdoption, AdoptionError> {
    if !observed.is_current(&snapshot.project, stack, scope) {
        return Err(AdoptionError::Stale);
    }
    let original = &observed.prepared;
    if source(conn, &snapshot.project, original.target.address())
        .ok()
        .as_ref()
        != Some(&original.source)
    {
        return Err(AdoptionError::SourceChanged);
    }
    if selected.is_empty() {
        return Err(AdoptionError::EmptySelection);
    }
    let mut seen = BTreeSet::new();
    // Validate selection identity before deriving any mutation.
    for &index in selected {
        if !seen.insert(index) {
            return Err(AdoptionError::DuplicateRow(index));
        }
        if index >= observed.rows.len() {
            return Err(AdoptionError::MissingRow(index));
        }
    }
    let mut values = Vec::new();
    let mut associations = Vec::new();
    for &index in selected {
        let row = &observed.rows[index];
        if row.kind == "association" {
            associations.push(index);
            continue;
        }
        if row.kind != "parameter" {
            return Err(AdoptionError::UnsupportedRow {
                index,
                kind: row.kind,
            });
        }
        let parameter = row
            .parameter
            .as_ref()
            .ok_or(AdoptionError::UnknownRow { index })?;
        if row.reason.is_some() || row.project.is_none() || row.device.is_none() {
            return Err(AdoptionError::UnknownRow { index });
        }
        if ambiguous(&original.source.image, parameter) {
            return Err(AdoptionError::AmbiguousParameter(
                parameter.reference.clone(),
            ));
        }
        values.push(ParameterAdoption {
            reference: parameter.reference.clone(),
            raw: row
                .device
                .clone()
                .ok_or(AdoptionError::UnknownRow { index })?,
        });
    }
    let mut links = LinkPlan::default();
    if !associations.is_empty() {
        // Activation under the candidate parameters, as the rebuild evaluates it.
        let program_id = &original.source.request.program_id;
        let mut candidate_values = original.source.request.values.clone();
        for value in &values {
            candidate_values.insert(value.reference.clone(), value.raw.clone());
        }
        let active = knx_productdb::image_request::active_unscoped_com_object_refs(
            conn,
            program_id,
            &candidate_values,
        )
        .map_err(|e| AdoptionError::Preparation(e.to_string()))?;
        links = plan_links(
            &snapshot.project,
            original.device,
            program_id,
            &active,
            &observed.rows,
            &associations,
        )?;
    }
    let command = PreparedComparisonAdoption::prepare_with_links(
        &snapshot.project,
        stack,
        original.device,
        original.revision,
        values,
        links.edits.clone(),
    )?;
    let (project, history) = command.candidate();
    // Existing image builder revalidates ranges, activation, unions, links and
    // encodings for the entire proposed configuration before accepting any edit.
    let candidate_source = source(conn, project, original.target.address())?;
    let candidate_rows = decode(
        &candidate_source.image,
        &observed.comparison.held,
        &candidate_source.plan,
    );
    check_candidate_links(&links, &candidate_rows)?;
    for &index in selected {
        let before = &observed.rows[index];
        if before.kind == "association" {
            continue;
        }
        let parameter = before
            .parameter
            .as_ref()
            .ok_or(AdoptionError::UnknownRow { index })?;
        let matches: Vec<_> = candidate_rows
            .iter()
            .filter(|r| {
                r.parameter
                    .as_ref()
                    .is_some_and(|p| same_placement(p, parameter))
            })
            .collect();
        if matches.len() != 1
            || matches[0].reason.is_some()
            || matches[0].project != before.device
            || matches[0].device != before.device
            || ambiguous(&candidate_source.image, parameter)
        {
            return Err(AdoptionError::CandidateFieldChanged(
                parameter.reference.clone(),
            ));
        }
    }
    let mut candidate = snapshot.clone();
    candidate.project = project.clone();
    preflight(&candidate, history)?;
    Ok(PreparedAdoption {
        before: snapshot.clone(),
        scope: scope.clone(),
        target: original.target.address(),
        original_source: original.source.clone(),
        candidate_source,
        command,
        rows: observed.rows.clone(),
        selected: selected.to_vec(),
        link_changes: links.changes,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionOutcome {
    pub changed: bool,
    pub native_generation: Option<i64>,
}
#[derive(Debug, Clone, Copy)]
pub struct AdoptionNativeTarget<'a> {
    pub path: &'a Path,
    pub history_generation: i64,
}

/// Caller holds project, context, stack and source ownership throughout this
/// non-async operation. Supply `native` for a saved project. A failed native
/// write leaves all live fields untouched; the same preview may be retried.
pub fn apply_adoption(
    conn: &Connection,
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    scope: &ComparisonScope,
    preview: &PreparedAdoption,
    native: Option<AdoptionNativeTarget<'_>>,
) -> Result<AdoptionOutcome, AdoptionError> {
    if *scope != preview.scope
        || *snapshot != preview.before
        || !preview.command.is_current(&snapshot.project, stack)
    {
        return Err(AdoptionError::Stale);
    }
    let (project, history) = preview.command.candidate();
    if source(conn, &snapshot.project, preview.target)
        .ok()
        .as_ref()
        != Some(&preview.original_source)
        || source(conn, project, preview.target).ok().as_ref() != Some(&preview.candidate_source)
    {
        return Err(AdoptionError::SourceChanged);
    }
    let mut candidate = snapshot.clone();
    candidate.project = project.clone();
    preflight(&candidate, history)?;
    if !preview.changed() {
        return Ok(AdoptionOutcome {
            changed: false,
            native_generation: None,
        });
    }
    let native_generation = native
        .map(|target| {
            let conn = knx_store::project_history::open_editor_store(target.path, false)
                .map_err(|e| AdoptionError::Native(e.to_string()))?;
            let before_hash = preview
                .before
                .semantic_hash()
                .map_err(|e| AdoptionError::Native(e.to_string()))?;
            knx_store::project_history::save_editor_if_unchanged(
                &conn,
                &candidate,
                history,
                target.history_generation,
                &before_hash,
                false,
            )
            .map_err(|e| AdoptionError::Native(e.to_string()))
        })
        .transpose()?;
    // The candidate was produced through do_command, never direct field edits.
    preview.command.apply(&mut snapshot.project, stack)?;
    Ok(AdoptionOutcome {
        changed: true,
        native_generation,
    })
}

#[cfg(test)]
mod link_tests;
#[cfg(test)]
mod tests;
