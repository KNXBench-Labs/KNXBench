//! Resolves the five written object flags without object size or priority admission.

use super::{activate_values, FlagOverrides, ImageError, ParameterTypes};
use crate::code::load_program_code;
use crate::query::ComObjectView;
use knx_core::commissioning::group_object_table::{
    GroupObjectTableError, ObjectFlags, TransmissionPriority,
};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

/// The five flags written by the currently admitted group-object table formats.
///
/// Read-on-init is deliberately absent: Easy3 discloses its loss and legacy
/// images refuse an enabled request. It is not a written false/default flag.
/// Priority is an image-admission concern, not an effective flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveObjectFlags {
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
}

impl From<EffectiveObjectFlags>
    for knx_core::commissioning::parameter_baseline::WrittenObjectFlags
{
    fn from(flags: EffectiveObjectFlags) -> Self {
        Self {
            read: flags.read,
            write: flags.write,
            transmit: flags.transmit,
            update: flags.update,
            communication: flags.communication,
            read_on_init: None,
        }
    }
}

/// Active reference provenance from the same checked pass that resolves its flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveObjectFlags {
    pub ref_id: String,
    pub flags: EffectiveObjectFlags,
}

impl EffectiveObjectFlags {
    pub(super) fn with_priority(self, priority: TransmissionPriority) -> ObjectFlags {
        ObjectFlags {
            read: self.read,
            write: self.write,
            transmit: self.transmit,
            update: self.update,
            communication: self.communication,
            priority,
        }
    }
}

/// Resolves active unscoped object flags under the supplied catalogue connection.
///
/// Missing choices take product defaults, just as in image construction. Uses
/// the image's checked activation and object/ref precedence; explicit overrides
/// apply only to active refs, after validating the product's flags. Uncertain
/// evaluation, module refs, unreadable flags/numbers and duplicate active numbers
/// are errors, not empty results. `Ok(empty)` means known to have no active objects.
///
/// This does not admit an image: object size, priority, placement and format
/// guards remain the builder's responsibility. Read-on-init is parsed to retain
/// the builder's refusal semantics, but is not returned as a written sixth flag.
/// Callers resolving a written request must use the same catalogue lease and
/// parameter choices as the image, not a subsequently changed catalogue.
pub fn resolve_effective_object_flags(
    conn: &Connection,
    program_id: &str,
    values: &BTreeMap<String, String>,
    overrides: &BTreeMap<String, FlagOverrides>,
) -> Result<BTreeMap<u8, EffectiveObjectFlags>, ImageError> {
    Ok(
        resolve_effective_object_flags_with_refs(conn, program_id, values, overrides)?
            .into_iter()
            .map(|(number, active)| (number, active.flags))
            .collect(),
    )
}

/// Effective flags with exact active reference identity, without a second evaluation.
pub fn resolve_effective_object_flags_with_refs(
    conn: &Connection,
    program_id: &str,
    values: &BTreeMap<String, String>,
    overrides: &BTreeMap<String, FlagOverrides>,
) -> Result<BTreeMap<u8, ActiveObjectFlags>, ImageError> {
    let code = load_program_code(conn, program_id)?
        .ok_or_else(|| ImageError::UnknownProgram(program_id.to_string()))?;
    let types = ParameterTypes::load(conn, program_id, &code)?.admitting_floats();
    for (reference, value) in values {
        types.value(reference, value)?;
    }
    let (resolved_values, activation) = activate_values(conn, program_id, values, &types)?;
    // A Background Assign can write an invisible controller after evaluation.
    // Share the image's consistency check before calling its object set known:
    // the activation must agree with the values the image would actually write.
    super::assigned_invisible_values(&code, &types, &activation, &resolved_values)?;
    let refs: Vec<&str> = activation
        .com_object_refs
        .iter()
        .map(|active| active.ref_id.as_str())
        .collect();
    let views = crate::query::com_object_views(conn, program_id, &refs, None)?;
    // Validate all numbers before collecting a number-keyed result; no winner
    // may hide another active ref even when their flags would be identical.
    let mut numbers = BTreeSet::new();
    let mut resolved = Vec::with_capacity(refs.len());
    for reference in refs {
        let view = views
            .get(reference)
            .ok_or_else(|| ImageError::Unsupported {
                what: format!("{reference}: no such ComObjectRef"),
            })?;
        let number = object_number(reference, view)?;
        if !numbers.insert(number) {
            return Err(ImageError::GroupObjects(
                GroupObjectTableError::DuplicateObject { number },
            ));
        }
        resolved.push((
            number,
            ActiveObjectFlags {
                ref_id: reference.to_string(),
                flags: resolve_flags(reference, view, overrides.get(reference))?.flags,
            },
        ));
    }
    Ok(resolved.into_iter().collect())
}

pub(super) fn object_number(reference: &str, view: &ComObjectView) -> Result<u8, ImageError> {
    view.number
        .and_then(|number| u8::try_from(number).ok())
        .ok_or_else(|| ImageError::Unsupported {
            what: format!("{reference}: object number {:?}", view.number),
        })
}

/// Read-on-init is a request used only by format-specific loss/refusal handling.
pub(super) struct ResolvedFlags {
    pub flags: EffectiveObjectFlags,
    pub requested_read_on_init: bool,
}

pub(super) fn resolve_flags(
    reference: &str,
    view: &ComObjectView,
    overrides: Option<&FlagOverrides>,
) -> Result<ResolvedFlags, ImageError> {
    let flag = |name: &str, value: &Option<String>| match value.as_deref() {
        Some("Enabled") => Ok(true),
        Some("Disabled") => Ok(false),
        other => Err(ImageError::Unsupported {
            what: format!("{reference}: {name} {other:?}"),
        }),
    };
    // Parse product flags even if overridden: an override does not broaden
    // image admission for an unreadable product declaration.
    let product_read_on_init = match view.read_on_init.as_deref() {
        None => false,
        Some(_) => flag("ReadOnInitFlag", &view.read_on_init)?,
    };
    let mut flags = EffectiveObjectFlags {
        update: flag("UpdateFlag", &view.update)?,
        transmit: flag("TransmitFlag", &view.transmit)?,
        write: flag("WriteFlag", &view.write)?,
        read: flag("ReadFlag", &view.read)?,
        communication: flag("CommunicationFlag", &view.communication)?,
    };
    if let Some(overrides) = overrides {
        overrides.apply(&mut flags);
    }
    Ok(ResolvedFlags {
        flags,
        requested_read_on_init: overrides
            .and_then(|overrides| overrides.read_on_init)
            .unwrap_or(product_read_on_init),
    })
}
