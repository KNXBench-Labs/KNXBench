//! Relative application source data stays separate from absolute image addresses.

use super::{verbatim, CodeError, Element, Parser};
use std::collections::BTreeMap;

/// One source `Static/Code/RelativeSegment`, not an allocated memory region.
///
/// This is a bounded source view, not full XML-schema validation. No runtime
/// base address, generated table, activation or executable plan is inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeSegment {
    /// The source identity, unchanged.
    pub id: String,
    /// The declared relative offset, not an absolute memory address.
    pub offset: u32,
    /// Declared octet count, not an allocation request.
    pub size: u32,
    /// Source load-state-machine index, not an inferred interface-object index.
    pub load_state_machine: u8,
    /// Base64-decoded source bytes; absent data is not synthesized.
    pub data: Option<Vec<u8>>,
    /// Base64-decoded uninterpreted mask, if present.
    pub mask: Option<Vec<u8>>,
    /// All other source attributes, including unknown ones, unchanged.
    pub other: BTreeMap<String, String>,
}

impl Parser<'_> {
    pub(super) fn read_relative_segment(
        &mut self,
        element: &Element,
    ) -> Result<RelativeSegment, CodeError> {
        let a = &element.attributes;
        let id = a
            .get("Id")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.malformed("a RelativeSegment has no nonempty Id"))?
            .to_string();
        let number = |name: &str| -> Result<u32, CodeError> {
            let text = a
                .get(name)
                .ok_or_else(|| self.malformed(format!("RelativeSegment {id} has no {name}")))?;
            text.parse()
                .map_err(|_| self.malformed(format!("RelativeSegment {id} has {name}={text:?}")))
        };
        let offset = number("Offset")?;
        let size = number("Size")?;
        let load_state_machine = u8::try_from(number("LoadStateMachine")?).map_err(|_| {
            self.malformed("RelativeSegment LoadStateMachine does not fit one octet")
        })?;
        let mut segment = RelativeSegment {
            id,
            offset,
            size,
            load_state_machine,
            data: None,
            mask: None,
            other: verbatim(a)
                .into_iter()
                .filter(|(name, _)| {
                    !matches!(name.as_str(), "Id" | "Offset" | "Size" | "LoadStateMachine")
                })
                .collect(),
        };
        if !element.empty {
            while let Some(child) = self.next_inside("RelativeSegment")? {
                let slot = match child.name.as_str() {
                    "Data" => &mut segment.data,
                    "Mask" => &mut segment.mask,
                    _ => {
                        return Err(self.malformed(format!(
                            "RelativeSegment {} has unsupported child {}",
                            segment.id, child.name
                        )));
                    }
                };
                if slot.is_some() {
                    return Err(self.malformed(format!(
                        "RelativeSegment {} has duplicate {}",
                        segment.id, child.name
                    )));
                }
                *slot = Some(self.read_segment_blob(
                    "RelativeSegment",
                    &segment.id,
                    segment.size,
                    &child,
                )?);
            }
        }
        Ok(segment)
    }
}
