//! Fail closed on source controls outside the explicitly bounded image grammar.

use super::{unsupported, ImageError};
use crate::xml::Attrs;

pub(super) fn validate(name: &str, path: &[String], attrs: &Attrs) -> Result<(), ImageError> {
    // Procedure translation has a separate owner/validator. The image consumes
    // its production metadata projection, not a second procedure interpreter.
    if path.iter().any(|p| p == "LoadProcedures") || name == "LoadProcedures" {
        return Ok(());
    }
    let parent = path.last().map(String::as_str).unwrap_or_default();
    let allowed = match (parent, name) {
        ("ApplicationPrograms", "ApplicationProgram") => "Id Name ApplicationNumber ApplicationVersion ProgramType MaskVersion VisibleDescription LoadProcedureStyle PeiType DefaultLanguage DynamicTableManagement Linkable MinEtsVersion PreEts4Style NonRegRelevantDataVersion Hash",
        ("ApplicationProgram", "Static" | "Dynamic") => "",
        ("Static", "Code" | "ParameterTypes" | "Parameters" | "ParameterRefs" | "ComObjectRefs") => "",
        ("Code", "AbsoluteSegment") => "Id Address Size",
        ("AbsoluteSegment", "Data" | "Mask") => "",
        ("ParameterTypes", "ParameterType") => "Id Name",
        ("ParameterType", "TypeNumber") => "SizeInBit Type minInclusive maxInclusive",
        ("ParameterType", "TypeRestriction") => "Base SizeInBit",
        ("ParameterType", "TypeNone") => "",
        ("TypeRestriction", "Enumeration") => "Id Text Value DisplayOrder BinaryValue",
        ("Parameters", "Union") => "SizeInBit",
        ("Parameters" | "Union", "Parameter") => "Id Name ParameterType Text Access Value Offset BitOffset DefaultUnionParameter LegacyPatchAlways",
        ("Union" | "Parameter", "Memory") => "CodeSegment Offset BitOffset",
        ("ParameterRefs", "ParameterRef") => "Id RefId Tag DisplayOrder Name Access Value Text",
        ("Static", "ComObjectTable") => "CodeSegment Offset",
        ("ComObjectTable", "ComObject") => "Id Name Text Number FunctionText ObjectSize ReadFlag WriteFlag CommunicationFlag TransmitFlag UpdateFlag ReadOnInitFlag DatapointType Priority",
        ("ComObjectRefs", "ComObjectRef") => "Id RefId Tag ObjectSize DatapointType Text FunctionText ReadFlag WriteFlag CommunicationFlag TransmitFlag UpdateFlag ReadOnInitFlag Priority",
        ("Static", "AddressTable" | "AssociationTable") => "CodeSegment Offset MaxEntries",
        ("Static", "Options") => "ParameterByteOrder DownloadInvisibleParameters",
        ("Dynamic" | "Channel" | "ParameterBlock" | "when", "Channel") => "Id Name Number Text",
        ("Dynamic" | "Channel" | "ParameterBlock" | "when", "ParameterBlock") => "Id Name Text ParamRefId",
        ("Dynamic" | "Channel" | "ParameterBlock" | "when", "choose") => "ParamRefId",
        ("choose", "when") => "test default",
        ("Dynamic" | "Channel" | "ParameterBlock" | "when", "ParameterRefRef" | "ComObjectRefRef") => "RefId",
        _ => return Err(unsupported(format!("unrecognized source element {parent}/{name}"))),
    };
    for attribute in attrs.names() {
        if !allowed.split_whitespace().any(|known| known == attribute) {
            return Err(unsupported(format!(
                "unrecognized source attribute {name}/@{attribute}"
            )));
        }
    }
    if name == "TypeNumber" {
        match attrs.get("Type") {
            Some("unsignedInt") => {
                for key in ["minInclusive", "maxInclusive"] {
                    if let Some(value) = attrs.get(key) {
                        value.parse::<u64>().map_err(|_| {
                            unsupported(format!("TypeNumber/@{key} is not unsigned"))
                        })?;
                    }
                }
            }
            Some("signedInt") => {
                for key in ["minInclusive", "maxInclusive"] {
                    if let Some(value) = attrs.get(key) {
                        value
                            .parse::<i64>()
                            .map_err(|_| unsupported(format!("TypeNumber/@{key} is not signed")))?;
                    }
                }
            }
            _ => return Err(unsupported("unrecognized TypeNumber type")),
        }
    }
    if name == "TypeRestriction" && !matches!(attrs.get("Base"), Some("Value" | "BinaryValue")) {
        return Err(unsupported("unrecognized TypeRestriction base"));
    }
    if name == "ApplicationProgram" {
        for (key, allowed) in [("DynamicTableManagement", "false"), ("Linkable", "false")] {
            if attrs.get(key).is_some_and(|value| value != allowed) {
                return Err(unsupported(format!(
                    "unsupported ApplicationProgram/@{key}"
                )));
            }
        }
    }
    Ok(())
}
