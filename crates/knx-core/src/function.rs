//! Project Functions: named groups of group addresses placed in one building part.
//!
//! Project Functions (MODEL-07, ADR-0124): a named group of group addresses
//! placed in one building part.
//!
//! *Project Schema23* §1.2.6.4/§1.2.6.7 lists `Function` children of a
//! `Space` ("List of functions in this building part"); §1.2.6.9 defines
//! `Function_t` and §1.2.6.10 its ordered `GroupAddressRef_t` members. A
//! Function owns nothing but its members: the building part and the group
//! addresses keep their own owners, and a Function only refers to them.
//!
//! Imported text is kept verbatim. [`FunctionDetails::validate`] and
//! [`FunctionMember::validate`] are the admission rules for *edited* values
//! only, the same split as `names::validate_edited_name`.

use std::collections::BTreeSet;

use crate::commissioning::CompletionStatus;
use crate::{BuildingPartId, GroupAddressId, InstallationId, Project, SourceRef};

/// `knx:String255_t` (Schema23 §1.2.6.9/§1.2.6.10): `Name`, `Type`,
/// `Number` and a member's `Name`/`Role`.
pub const MAX_FUNCTION_SHORT_TEXT_CODEPOINTS: usize = 255;
/// `Comment`/`Description` are unbounded `xs:string`; edited values share
/// the project-note body budget.
pub const MAX_FUNCTION_LONG_TEXT_BYTES: usize = 64 * 1024;

pub use crate::ids::{FunctionId, FunctionMemberId};

/// One `GroupAddressRef`: a group address in its Function's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionMember {
    pub id: FunctionMemberId,
    /// `GroupAddressRef/@Id`; empty for a member created in KNXBench.
    pub source: SourceRef,
    pub group_address: GroupAddressId,
    /// Required by the schema, and empty in the measured ETS 6.1 instance.
    pub name: String,
    /// Free text such as `SwitchOnOff`, verbatim; never interpreted.
    pub role: Option<String>,
}

/// The editable text and status of a Function, without identity,
/// placement or members.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionDetails {
    pub name: String,
    /// `Function/@Type` verbatim (the measured instance holds a master-data
    /// function-type id such as `FT-1`); not resolved here.
    pub function_type: Option<String>,
    pub number: Option<String>,
    pub comment: Option<String>,
    pub description: Option<String>,
    pub completion: CompletionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: FunctionId,
    /// `Function/@Id`; empty for a Function created in KNXBench.
    pub source: SourceRef,
    /// The building part (`Space`) that lists this Function.
    pub location: BuildingPartId,
    pub details: FunctionDetails,
    /// In source order; the order is meaningful and preserved.
    pub members: Vec<FunctionMember>,
}

/// Why an edited Function value is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionError {
    InvalidName(crate::names::NameError),
    /// A `String255_t` field is longer than 255 code points, or a long text
    /// exceeds its byte budget.
    TooLong(&'static str),
    ControlCharacter(&'static str),
    /// The group address is already a member of this Function.
    DuplicateMember(GroupAddressId),
    InvalidPosition(usize),
}

impl std::fmt::Display for FunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(e) => write!(f, "{e}"),
            Self::TooLong(field) => write!(f, "function {field} is too long"),
            Self::ControlCharacter(field) => {
                write!(f, "function {field} must not contain control characters")
            }
            Self::DuplicateMember(id) => {
                write!(f, "group address {id} is already part of this function")
            }
            Self::InvalidPosition(position) => write!(f, "position {position} is invalid"),
        }
    }
}

impl std::error::Error for FunctionError {}

fn short_text(field: &'static str, value: &str) -> Result<(), FunctionError> {
    if value.chars().count() > MAX_FUNCTION_SHORT_TEXT_CODEPOINTS {
        return Err(FunctionError::TooLong(field));
    }
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(FunctionError::ControlCharacter(field));
    }
    Ok(())
}

fn long_text(field: &'static str, value: &str) -> Result<(), FunctionError> {
    if value.len() > MAX_FUNCTION_LONG_TEXT_BYTES {
        return Err(FunctionError::TooLong(field));
    }
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
    {
        return Err(FunctionError::ControlCharacter(field));
    }
    Ok(())
}

impl FunctionDetails {
    /// Admission for an edited value: a non-blank name (the shared
    /// edited-name rule) of at most 255 code points, single-line short
    /// fields, bounded long texts. Text is stored exactly as given.
    pub fn validate(&self) -> Result<(), FunctionError> {
        crate::names::validate_edited_name(&self.name).map_err(FunctionError::InvalidName)?;
        short_text("name", &self.name)?;
        for (field, value) in [("type", &self.function_type), ("number", &self.number)] {
            if let Some(value) = value {
                short_text(field, value)?;
            }
        }
        for (field, value) in [
            ("comment", &self.comment),
            ("description", &self.description),
        ] {
            if let Some(value) = value {
                long_text(field, value)?;
            }
        }
        Ok(())
    }
}

impl FunctionMember {
    /// Admission for an edited member. The name may be empty, as ETS
    /// itself writes it (`Name=""` in the measured instance).
    pub fn validate(&self) -> Result<(), FunctionError> {
        short_text("member name", &self.name)?;
        if let Some(role) = &self.role {
            short_text("member role", role)?;
        }
        Ok(())
    }
}

/// A stored or imported Function set that breaks an ownership or
/// reference rule. Commands cannot produce these; a file or an import can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionInvariantError {
    ZeroId,
    DuplicateFunctionId(FunctionId),
    DuplicateMemberId(FunctionMemberId),
    /// The location is missing or exists in more than one installation.
    UnplacedLocation {
        function: FunctionId,
        location: BuildingPartId,
    },
    /// The member's group address is missing, ambiguous, or owned by an
    /// installation other than the location's.
    ForeignMember {
        function: FunctionId,
        member: FunctionMemberId,
        group_address: GroupAddressId,
    },
}

impl std::fmt::Display for FunctionInvariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroId => write!(f, "function or member id 0 is reserved"),
            Self::DuplicateFunctionId(id) => write!(f, "function id {id} occurs twice"),
            Self::DuplicateMemberId(id) => write!(f, "function member id {id} occurs twice"),
            Self::UnplacedLocation { function, location } => write!(
                f,
                "function {function} is placed in building part {location}, which is missing or ambiguous"
            ),
            Self::ForeignMember {
                function,
                member,
                group_address,
            } => write!(
                f,
                "member {member} of function {function} refers to group address {group_address}, \
                 which is missing, ambiguous or in another installation"
            ),
        }
    }
}

impl std::error::Error for FunctionInvariantError {}

/// The installation that owns `part`, if exactly one does.
pub fn location_installation(project: &Project, part: BuildingPartId) -> Option<InstallationId> {
    let mut owners = project
        .installations
        .iter()
        .filter(|i| i.buildings.iter().any(|b| b.id == part));
    let owner = owners.next()?;
    owners.next().is_none().then_some(owner.id)
}

/// The installation that owns `address`, if exactly one does.
pub fn group_address_installation(
    project: &Project,
    address: GroupAddressId,
) -> Option<InstallationId> {
    let mut owners = project
        .installations
        .iter()
        .filter(|i| i.group_addresses.iter().any(|g| g.id == address));
    let owner = owners.next()?;
    owners.next().is_none().then_some(owner.id)
}

/// Checks every ownership and reference rule of ADR-0124 over a whole
/// project: unique non-zero IDs, a location owned by exactly one
/// installation, and members whose group address lives in that same
/// installation.
pub fn validate_functions(project: &Project) -> Result<(), FunctionInvariantError> {
    let mut functions = BTreeSet::new();
    let mut members = BTreeSet::new();
    for function in &project.functions {
        if function.id.0 == 0 {
            return Err(FunctionInvariantError::ZeroId);
        }
        if !functions.insert(function.id) {
            return Err(FunctionInvariantError::DuplicateFunctionId(function.id));
        }
        let Some(owner) = location_installation(project, function.location) else {
            return Err(FunctionInvariantError::UnplacedLocation {
                function: function.id,
                location: function.location,
            });
        };
        for member in &function.members {
            if member.id.0 == 0 {
                return Err(FunctionInvariantError::ZeroId);
            }
            if !members.insert(member.id) {
                return Err(FunctionInvariantError::DuplicateMemberId(member.id));
            }
            if group_address_installation(project, member.group_address) != Some(owner) {
                return Err(FunctionInvariantError::ForeignMember {
                    function: function.id,
                    member: member.id,
                    group_address: member.group_address,
                });
            }
        }
    }
    Ok(())
}

/// Whether any Function lists `address` as a member.
pub fn group_address_in_function(project: &Project, address: GroupAddressId) -> bool {
    project
        .functions
        .iter()
        .flat_map(|f| &f.members)
        .any(|m| m.group_address == address)
}

/// Whether any Function is placed in `part`.
pub fn building_part_has_functions(project: &Project, part: BuildingPartId) -> bool {
    project.functions.iter().any(|f| f.location == part)
}
