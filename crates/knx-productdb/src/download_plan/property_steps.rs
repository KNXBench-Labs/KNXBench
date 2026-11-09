//! Plans mask `0701h` master resets and parameter-filled property writes (ADR-0161/0162/0167).
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
//! - With an all-zero `InlineData` of at least `n` octets as well, ETS read
//!   nothing and wrote zero into every element no parameter reaches; with a
//!   shorter one it read and merged the device's value as without; a
//!   property no parameter reaches it did not write at all
//!   (docs/research/2026-11-08-time-parameters-and-inline-property-writes.md,
//!   ADR-0162). A non-zero `InlineData` was not observed and is refused.
//! - Without `Count` and `InlineData`, ETS wrote the declared `MaxElements`,
//!   read nothing and wrote zero into every bit no parameter owns (ADR-0167).

use std::collections::{BTreeMap, BTreeSet};

use knx_core::commissioning::load_state::MaskVersion;
use knx_core::commissioning::memory_download::MemoryDownloadStep;

use super::DownloadPlanError;
use crate::image::DownloadImage;
use crate::inference::Inference;

/// [`Inference::rule`]: a master reset is sent as the function property
/// command ETS 6.4 maps it to.
pub const MASTER_RESET_AS_FUNCTION_PROPERTY: &str = "master-reset-as-function-property";

/// [`Inference::rule`]: a parameter-filled write without `Count` writes the
/// declared `MaxElements`, zero where no parameter reaches; nothing is read.
pub const DECLARED_SIZE_WITHOUT_COUNT: &str = "declared-size-without-count";

/// [`Inference::rule`]: an all-zero `InlineData` covering `Count` fills
/// every element no parameter reaches; nothing is read.
pub const INLINE_DATA_FILLS_PROPERTY: &str = "inline-data-fills-property";

/// [`Inference::rule`]: an `InlineData` shorter than `Count` is not used;
/// the device's value is read and merged.
pub const SHORT_INLINE_DATA_READS_DEVICE: &str = "short-inline-data-reads-device";

/// [`Inference::rule`]: a parameter-filled write with `InlineData` that
/// reaches no parameter is not sent.
pub const PROPERTY_WITHOUT_PARAMETERS_NOT_SENT: &str = "property-without-parameters-not-sent";

const REFERENCE: &str = "docs/research/2026-11-08-property-placed-parameters.md";
const INLINE_REFERENCE: &str =
    "docs/research/2026-11-08-time-parameters-and-inline-property-writes.md";
const DECLARED_REFERENCE: &str = "docs/research/2026-11-08-property-writes-without-a-count.md";

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
    /// Declared properties and their `MaxElements`.
    declared: BTreeMap<(u8, u8), u16>,
    written: BTreeSet<(u8, u8)>,
    /// Writes without `Count`, sized by the declaration, without a read.
    declared_size: usize,
    /// Writes filled from their `InlineData`, without a read.
    inline_filled: usize,
    /// Writes whose `InlineData` is shorter than `Count`.
    inline_short: usize,
    /// Writes with `InlineData` that reach no parameter, so send nothing.
    unsent: usize,
}

impl<'a> PropertySteps<'a> {
    pub(super) fn new(image: &'a DownloadImage, mask: MaskVersion) -> Self {
        Self {
            image,
            mask,
            declared: BTreeMap::new(),
            written: BTreeSet::new(),
            declared_size: 0,
            inline_filled: 0,
            inline_short: 0,
            unsent: 0,
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
        (object_index, property_id): (u8, u8),
        property_type: &str,
        max_elements: u16,
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
        self.declared
            .insert((object_index, property_id), max_elements);
        Ok(())
    }

    /// The step a parameter-filled write becomes; `None` when, as ETS 6.4
    /// did, it sends nothing.
    pub(super) fn write(
        &mut self,
        object_index: u8,
        property_id: u8,
        count: Option<u16>,
        inline_data: Option<&[u8]>,
    ) -> Result<Option<MemoryDownloadStep>, DownloadPlanError> {
        let refuse = |reason: &str| DownloadPlanError::PropertyParameters {
            object_index,
            property_id,
            reason: reason.to_string(),
        };
        let key = (object_index, property_id);
        let Some(&max_elements) = self.declared.get(&key) else {
            return Err(refuse(
                "written by parameters without a declared description",
            ));
        };
        if !self.written.insert(key) {
            return Err(refuse("written by parameters twice"));
        }
        let image = self.image;
        let property = image
            .properties
            .iter()
            .find(|property| (property.object_index, property.property_id) == key);
        let Some(count) = count else {
            return self.write_declared(key, max_elements, property.map(|p| &p.octets[..]));
        };
        if count == 0 {
            return Err(refuse("written with Count=0"));
        }
        if inline_data.is_some_and(|inline| inline.iter().any(|octet| *octet != 0)) {
            return Err(refuse(
                "InlineData with a non-zero octet; only all-zero InlineData was observed",
            ));
        }
        let elements = usize::from(count);
        let (mut octets, mut owned) = match (property, inline_data) {
            (Some(property), _) => (property.octets.clone(), property.owned.clone()),
            (None, Some(_)) => {
                self.unsent += 1;
                return Ok(None);
            }
            (None, None) => Default::default(),
        };
        if octets.len() > elements {
            return Err(refuse("parameters lie past the elements Count writes"));
        }
        octets.resize(elements, 0);
        owned.resize(elements, 0);
        match inline_data {
            // Every bit is the product's: zero where no parameter reaches.
            Some(inline) if inline.len() >= elements => {
                self.inline_filled += 1;
                owned = vec![0xFF; elements];
            }
            Some(_) => self.inline_short += 1,
            None => {}
        }
        Ok(Some(MemoryDownloadStep::WritePropertyParameters {
            object_index,
            property_id,
            elements: count,
            octets,
            owned,
        }))
    }

    /// A write without `Count` or `InlineData`: `[V]` ETS 6.4 wrote the
    /// declared `MaxElements`, read nothing, and wrote zero into every bit no
    /// parameter owns (ADR-0167). Only a property that holds parameters was
    /// observed.
    fn write_declared(
        &mut self,
        (object_index, property_id): (u8, u8),
        max_elements: u16,
        parameters: Option<&[u8]>,
    ) -> Result<Option<MemoryDownloadStep>, DownloadPlanError> {
        let refuse = |reason: &str| DownloadPlanError::PropertyParameters {
            object_index,
            property_id,
            reason: reason.to_string(),
        };
        let Some(parameters) = parameters else {
            return Err(refuse(
                "written without Count, but holds no parameter; that was not observed",
            ));
        };
        let elements = usize::from(max_elements);
        if elements == 0 {
            return Err(refuse(
                "written without Count, and declared with MaxElements=0",
            ));
        }
        if parameters.len() > elements {
            return Err(refuse("parameters lie past the declared MaxElements"));
        }
        let mut octets = parameters.to_vec();
        octets.resize(elements, 0);
        self.declared_size += 1;
        Ok(Some(MemoryDownloadStep::WritePropertyParameters {
            object_index,
            property_id,
            elements: max_elements,
            octets,
            owned: vec![0xFF; elements],
        }))
    }

    /// Every property the image places parameters in is written; the
    /// `InlineData` rules applied are named.
    pub(super) fn finish(self, inferences: &mut Vec<Inference>) -> Result<(), DownloadPlanError> {
        if let Some(property) = self.image.properties.iter().find(|property| {
            !self
                .written
                .contains(&(property.object_index, property.property_id))
        }) {
            return Err(DownloadPlanError::PropertyParameters {
                object_index: property.object_index,
                property_id: property.property_id,
                reason: "holds parameters, but the procedure never writes it".into(),
            });
        }
        for (count, rule, detail, reference) in [
            (
                self.declared_size,
                DECLARED_SIZE_WITHOUT_COUNT,
                "parameter-filled property writes carry no Count: the declared MaxElements are \
                 written, every bit no parameter owns as zero, and nothing is read, as ETS 6.4 \
                 did",
                DECLARED_REFERENCE,
            ),
            (
                self.inline_filled,
                INLINE_DATA_FILLS_PROPERTY,
                "parameter-filled property writes carry an all-zero InlineData covering Count: \
                 every element no parameter reaches is written as zero and nothing is read, as \
                 ETS 6.4 did",
                INLINE_REFERENCE,
            ),
            (
                self.inline_short,
                SHORT_INLINE_DATA_READS_DEVICE,
                "parameter-filled property writes carry an InlineData shorter than Count: it is \
                 not used, the device's value is read and only the parameters' bits change, as \
                 ETS 6.4 did",
                INLINE_REFERENCE,
            ),
            (
                self.unsent,
                PROPERTY_WITHOUT_PARAMETERS_NOT_SENT,
                "parameter-filled property writes with InlineData reach no parameter and are not \
                 sent, as ETS 6.4 did",
                INLINE_REFERENCE,
            ),
        ] {
            if count > 0 {
                inferences.push(Inference {
                    rule,
                    detail: format!("{count} {detail}"),
                    reference,
                });
            }
        }
        Ok(())
    }
}
