//! Images of the parameters a product places in interface-object properties (ADR-0161).
//!
//! `[V]` ETS 6.4, downloading a mask-`0701h` program with
//! `DownloadInvisibleParameters="Background"` to KNXBench's simulator, wrote
//! every parameter placed in a property: a visible one with its value, an
//! invisible one with its product default, numbers low octet first under
//! `ParameterByteOrder="LittleEndian"`. Every element no parameter reaches it
//! wrote back as the device held it
//! (docs/research/2026-11-08-property-placed-parameters.md). So an image here
//! is the parameters' bits plus a mask of which bits they own; the planner
//! and executor merge it into what the device holds.

use std::collections::BTreeMap;

use knx_core::commissioning::parameter_image::{reverse_octets, ParameterField, ParameterImage};

use super::{Encoded, ImageError, ParameterTypes};
use crate::code::{ParameterPlacement, ProgramCode};
use crate::dynamic::ValueMap;
use crate::inference::Inference;

/// [`Inference::rule`]: the parameters a product places in properties are
/// written into what the device holds, bit by bit.
pub const PROPERTY_PARAMETERS_MERGED: &str = "property-parameters-merged-into-device-value";

/// [`Inference::rule`]: an invisible parameter placed in a property is
/// written with its product default.
pub const INVISIBLE_PROPERTY_PARAMETERS_DEFAULT: &str = "invisible-property-parameters-default";

const REFERENCE: &str = "docs/research/2026-11-08-property-placed-parameters.md";

/// The parameters' bits in one property's value, from element 1 on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyImage {
    /// `ObjectIndex`.
    pub object_index: u8,
    /// `PropertyId`.
    pub property_id: u8,
    /// The parameters' bits; every other bit is zero.
    pub octets: Vec<u8>,
    /// Which bits of `octets` a parameter owns; as long as `octets`.
    pub owned: Vec<u8>,
}

/// One parameter's field in a property, its value as written, and its id.
type PlacedValue<'a> = (ParameterField, u64, &'a str);

/// Every property image `code` needs, keyed by `(object, property)`.
///
/// `written` holds the active parameters' values by `Parameter` id;
/// `defaults` the product defaults, for the invisible ones. Refuses unless
/// the program declares the policy ETS was observed with, for a layout whose
/// byte order is established.
pub(super) fn property_images(
    code: &ProgramCode,
    types: &ParameterTypes,
    written: &BTreeMap<String, (String, Encoded)>,
    defaults: &ValueMap,
    low_octet_first: bool,
    inferences: &mut Vec<Inference>,
) -> Result<Vec<PropertyImage>, ImageError> {
    let placed: Vec<_> = code
        .parameters
        .iter()
        .filter_map(|(parameter, placement)| match placement {
            ParameterPlacement::Property(property) => Some((parameter.as_str(), property)),
            _ => None,
        })
        .collect();
    if placed.is_empty() {
        return Ok(Vec::new());
    }
    // Without the observed policy, nothing is known about invisible ones:
    // a visible one is refused as before, invisible ones stay unwritten.
    if code
        .options
        .get("DownloadInvisibleParameters")
        .map(String::as_str)
        != Some("Background")
    {
        return match placed
            .iter()
            .find(|(parameter, _)| written.contains_key(*parameter))
        {
            Some((parameter, _)) => Err(ImageError::Unsupported {
                what: format!("{parameter}: placed by Property"),
            }),
            None => Ok(Vec::new()),
        };
    }
    let mut refs_of: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (parameter_ref, parameter) in types.refs() {
        refs_of.entry(parameter).or_default().push(parameter_ref);
    }

    let mut fields: BTreeMap<(u8, u8), Vec<PlacedValue<'_>>> = BTreeMap::new();
    let mut invisible = 0usize;
    for (parameter, property) in placed {
        let refs = refs_of
            .get(parameter)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let Some(&first_ref) = refs.first() else {
            return Err(ImageError::Unsupported {
                what: format!("{parameter}: placed in a property, but no ParameterRef names it"),
            });
        };
        let value = match written.get(parameter) {
            Some((_, value)) => value.clone(),
            None => {
                invisible += 1;
                default_of(parameter, refs, types, defaults)?
            }
        };
        let Encoded::Number(number) = value else {
            return Err(ImageError::Unsupported {
                what: format!("{parameter}: a text placed in a property has no observed encoding"),
            });
        };
        let field = ParameterField {
            offset: property.offset,
            bit_offset: property.bit_offset,
            size_in_bit: types.size(first_ref)?,
        };
        let ordered = if low_octet_first && u32::from(field.bit_offset) + field.size_in_bit > 8 {
            reverse_octets(field, number).map_err(|_| ImageError::Unsupported {
                what: format!(
                    "{parameter}: ParameterByteOrder \"LittleEndian\" field at bit offset {} \
                     with {} bits has no established octet order",
                    field.bit_offset, field.size_in_bit
                ),
            })?
        } else {
            number
        };
        fields
            .entry((property.object_index, property.property_id))
            .or_default()
            .push((field, ordered, parameter));
    }

    let mut images = Vec::with_capacity(fields.len());
    for ((object_index, property_id), fields) in fields {
        let length = fields
            .iter()
            .map(|(field, ..)| {
                field.offset as usize
                    + (u32::from(field.bit_offset) + field.size_in_bit).div_ceil(8) as usize
            })
            .max()
            .unwrap_or_default();
        let mut image = ParameterImage::new(vec![0; length]);
        for (field, value, parameter) in fields {
            image
                .write(field, value)
                .map_err(|error| ImageError::Unsupported {
                    what: format!(
                        "{parameter} in property {object_index}/{property_id}: {error:?}"
                    ),
                })?;
        }
        let owned = image.written_mask();
        images.push(PropertyImage {
            object_index,
            property_id,
            octets: image.into_octets(),
            owned,
        });
    }
    inferences.push(Inference {
        rule: PROPERTY_PARAMETERS_MERGED,
        detail: format!(
            "{} properties carry parameters; only the parameters' bits are written, every \
             other bit is written back as the device holds it",
            images.len()
        ),
        reference: REFERENCE,
    });
    if invisible > 0 {
        inferences.push(Inference {
            rule: INVISIBLE_PROPERTY_PARAMETERS_DEFAULT,
            detail: format!(
                "{invisible} invisible parameters placed in properties are written with their \
                 product default, as ETS 6.4 does under DownloadInvisibleParameters=\"Background\""
            ),
            reference: REFERENCE,
        });
    }
    Ok(images)
}

/// The product default of an invisible `parameter`: every `ParameterRef`
/// of it must give the same one.
fn default_of(
    parameter: &str,
    refs: &[&str],
    types: &ParameterTypes,
    defaults: &ValueMap,
) -> Result<Encoded, ImageError> {
    let mut found: Option<Encoded> = None;
    for parameter_ref in refs {
        let raw = defaults
            .get_unscoped(parameter_ref)
            .ok_or_else(|| ImageError::Value {
                parameter_ref: (*parameter_ref).to_string(),
                cause: "no default".to_string(),
            })?;
        let value = types.value(parameter_ref, raw)?;
        match &found {
            Some(earlier) if *earlier != value => {
                return Err(ImageError::Unsupported {
                    what: format!(
                        "{parameter}: invisible, and its ParameterRefs give different defaults"
                    ),
                })
            }
            _ => found = Some(value),
        }
    }
    found.ok_or_else(|| ImageError::Unsupported {
        what: format!("{parameter}: no default"),
    })
}
