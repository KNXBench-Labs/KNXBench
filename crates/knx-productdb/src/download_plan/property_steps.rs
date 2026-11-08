//! Translates master resets and parameter-filled property writes of mask `0701h` (ADR-0161).
//!
//! `[V]` What ETS 6.4 sent for these load controls to KNXBench's simulator
//! (docs/research/2026-11-08-property-placed-parameters.md):
//!
//! - `LdCtrlMasterReset EraseCode="6"` / `"5"` (channel 0) became
//!   `A_FunctionPropertyCommand` to object 5, PID 52, with `04h` / `10h`;
//!   ETS logged the mapping itself ("Mapping MasterReset(6) to
//!   WriteFunctionProperty(5, 52, 4)"). Object 5 is mask `0701h`'s first
//!   application object (`FirstAppObjectIdx` in the KNX master data). No
//!   other erase code, channel or mask was observed, so none is translated.
//! - `LdCtrlDeclarePropDesc` sent nothing: ETS read no description of a
//!   declared property and wrote it with the declared type.
//! - A parameter-filled `LdCtrlWriteProp Count="n"` wrote `n` elements from
//!   index 1, even where `n` exceeds the declared `MaxElements`.

use std::collections::BTreeSet;

use knx_core::commissioning::load_state::MaskVersion;
use knx_core::commissioning::memory_download::MemoryDownloadStep;

use super::DownloadPlanError;
use crate::image::DownloadImage;
use crate::inference::Inference;

/// [`Inference::rule`]: a master reset is sent as the function property
/// command ETS 6.4 maps it to.
pub const MASTER_RESET_AS_FUNCTION_PROPERTY: &str = "master-reset-as-function-property";

const REFERENCE: &str = "docs/research/2026-11-08-property-placed-parameters.md";

/// `FirstAppObjectIdx` of mask `0701h` in the KNX master data.
const FIRST_APPLICATION_OBJECT_0701: u8 = 5;
/// The function property ETS 6.4 sent the master reset to.
const MASTER_RESET_PROPERTY: u8 = 52;
/// The only declared type observed: one octet per element.
const UNSIGNED_CHAR: &str = "PDT_UNSIGNED_CHAR";

/// The command octet ETS 6.4 sent for an erase code on channel 0.
fn master_reset_command(erase_code: u8) -> Option<u8> {
    match erase_code {
        // Reset links.
        6 => Some(0x04),
        // Reset parameters.
        5 => Some(0x10),
        _ => None,
    }
}

/// The property steps of one plan, checked against each other and the image.
pub(super) struct PropertySteps<'a> {
    image: &'a DownloadImage,
    mask: MaskVersion,
    declared: BTreeSet<(u8, u8)>,
    written: BTreeSet<(u8, u8)>,
}

impl<'a> PropertySteps<'a> {
    pub(super) fn new(image: &'a DownloadImage, mask: MaskVersion) -> Self {
        Self {
            image,
            mask,
            declared: BTreeSet::new(),
            written: BTreeSet::new(),
        }
    }

    pub(super) fn master_reset(
        &self,
        erase_code: u8,
        channel: u8,
        inferences: &mut Vec<Inference>,
    ) -> Result<MemoryDownloadStep, DownloadPlanError> {
        let command = master_reset_command(erase_code)
            .filter(|_| channel == 0 && self.mask == MaskVersion(0x0701))
            .ok_or(DownloadPlanError::MasterReset {
                erase_code,
                channel,
            })?;
        inferences.push(Inference {
            rule: MASTER_RESET_AS_FUNCTION_PROPERTY,
            detail: format!(
                "LdCtrlMasterReset EraseCode={erase_code} is sent as A_FunctionPropertyCommand \
                 {FIRST_APPLICATION_OBJECT_0701}/{MASTER_RESET_PROPERTY} {command:02X}h, as ETS \
                 6.4 maps it for mask 0701h"
            ),
            reference: REFERENCE,
        });
        Ok(MemoryDownloadStep::MasterResetCommand {
            erase_code,
            object_index: FIRST_APPLICATION_OBJECT_0701,
            property_id: MASTER_RESET_PROPERTY,
            command,
        })
    }

    pub(super) fn declare(
        &mut self,
        object_index: u8,
        property_id: u8,
        property_type: &str,
        writable: bool,
    ) -> Result<(), DownloadPlanError> {
        let refuse = |reason: String| DownloadPlanError::PropertyParameters {
            object_index,
            property_id,
            reason,
        };
        if property_type != UNSIGNED_CHAR {
            return Err(refuse(format!(
                "declared {property_type}; only {UNSIGNED_CHAR} was observed"
            )));
        }
        if !writable {
            return Err(refuse("declared not writable".into()));
        }
        self.declared.insert((object_index, property_id));
        Ok(())
    }

    pub(super) fn write(
        &mut self,
        object_index: u8,
        property_id: u8,
        count: u16,
    ) -> Result<MemoryDownloadStep, DownloadPlanError> {
        let refuse = |reason: &str| DownloadPlanError::PropertyParameters {
            object_index,
            property_id,
            reason: reason.to_string(),
        };
        let key = (object_index, property_id);
        if !self.declared.contains(&key) {
            return Err(refuse(
                "written by parameters without a declared description",
            ));
        }
        if !self.written.insert(key) {
            return Err(refuse("written by parameters twice"));
        }
        if count == 0 {
            return Err(refuse("written with Count=0"));
        }
        let elements = usize::from(count);
        let (mut octets, mut owned) = self
            .image
            .properties
            .iter()
            .find(|property| (property.object_index, property.property_id) == key)
            .map(|property| (property.octets.clone(), property.owned.clone()))
            .unwrap_or_default();
        if octets.len() > elements {
            return Err(refuse("parameters lie past the elements Count writes"));
        }
        octets.resize(elements, 0);
        owned.resize(elements, 0);
        Ok(MemoryDownloadStep::WritePropertyParameters {
            object_index,
            property_id,
            elements: count,
            octets,
            owned,
        })
    }

    /// Every property the image places parameters in is written.
    pub(super) fn finish(self) -> Result<(), DownloadPlanError> {
        match self.image.properties.iter().find(|property| {
            !self
                .written
                .contains(&(property.object_index, property.property_id))
        }) {
            Some(property) => Err(DownloadPlanError::PropertyParameters {
                object_index: property.object_index,
                property_id: property.property_id,
                reason: "holds parameters, but the procedure never writes it".into(),
            }),
            None => Ok(()),
        }
    }
}
