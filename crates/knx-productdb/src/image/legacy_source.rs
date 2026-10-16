//! Source-bound legacy image checks with explicit family selection.

use std::collections::BTreeSet;

use super::{com_object_span_with_layout, placed, Encoded, ImageError, ImageLayout};
use crate::code::{ParameterPlacement, ProgramCode};
use knx_core::commissioning::parameter_image::{ParameterField, ParameterImage};

/// Corroborate inferred placements; explicit single-octet numbers need no endian guess.
pub(super) fn validate_default(
    code: &ProgramCode,
    layout: ImageLayout,
    segment: &str,
    field: ParameterField,
    reference: &str,
    placement: &ParameterPlacement,
    values: (&Encoded, &Encoded),
) -> Result<(), ImageError> {
    let (default, requested) = values;
    // Cookbook Load Controls p7 permits replacing template defaults. Schema23
    // BitOffset_t pp29–30 fixes explicit MSB placement within an octet, so
    // these fields need neither source-default equality nor a byte-order guess.
    // Union arithmetic and fields crossing octets still require corroboration.
    let explicit_single_octet = matches!(placement, ParameterPlacement::Memory(_))
        && field.bit_offset <= 7
        && (1..=8 - u32::from(field.bit_offset)).contains(&field.size_in_bit)
        && matches!(default, Encoded::Number(_));
    let family = layout.family();
    let unsupported = |reason: &str| ImageError::Unsupported {
        what: format!("{reference}: {family} {reason}"),
    };
    let base = code
        .segment(segment)
        .and_then(|s| s.data.as_ref())
        .ok_or_else(|| unsupported("parameter has no source bytes"))?;
    let mut probe = ParameterImage::new(base.clone());
    match default {
        Encoded::Number(value) => probe.write(field, *value),
        Encoded::Text { octets, .. } => probe.write_octets(field, octets),
    }
    .map_err(|error| ImageError::Parameter {
        parameter_ref: reference.into(),
        error,
    })?;
    if !explicit_single_octet && probe.into_octets() != *base {
        return Err(unsupported("default disagrees with source bits"));
    }
    // Without an explicit byte order, a symmetric default cannot corroborate
    // the byte order of an edited multi-octet number. Leave unchanged defaults
    // intact, but refuse speculative edits instead of calling zero an oracle.
    if default != requested
        && field.size_in_bit > 8
        && code.options.get("ParameterByteOrder").map(String::as_str) != Some("BigEndian")
    {
        if let Encoded::Number(value) = default {
            if field.bit_offset != 0 || !field.size_in_bit.is_multiple_of(8) {
                return Err(unsupported(
                    "edited multi-octet field has no byte-order witness",
                ));
            }
            let width = (field.size_in_bit / 8) as usize;
            let octets = value.to_be_bytes();
            let octets = &octets[8 - width..];
            if octets.iter().eq(octets.iter().rev()) {
                return Err(unsupported(
                    "edited multi-octet default is byte-order ambiguous",
                ));
            }
        }
    }
    Ok(())
}

/// Strict source regions of all three tables, not a hardware placement claim.
pub(super) fn table_spans(
    code: &ProgramCode,
    layout: ImageLayout,
) -> Result<Vec<(String, usize, usize)>, ImageError> {
    let family = layout.family();
    let unsupported = |what: &str| ImageError::Unsupported {
        what: format!("{family} {what}"),
    };
    let mut ids = BTreeSet::new();
    let mut ranges = Vec::new();
    for segment in &code.segments {
        let end = segment
            .address
            .checked_add(segment.size)
            .filter(|end| *end <= 0x1_0000)
            .ok_or_else(|| unsupported("segment exceeds address space"))?;
        if segment.size == 0
            || !ids.insert(&segment.id)
            || segment
                .data
                .as_ref()
                .is_some_and(|data| data.len() != segment.size as usize)
            || segment
                .mask
                .as_ref()
                .is_some_and(|mask| mask.len() != segment.size as usize)
        {
            return Err(unsupported(
                "segment has duplicate identity or inconsistent length",
            ));
        }
        if ranges
            .iter()
            .any(|(start, stop)| segment.address < *stop && *start < end)
        {
            return Err(unsupported("absolute source segments overlap"));
        }
        ranges.push((segment.address, end));
    }
    let object = com_object_span_with_layout(code, layout)?
        .ok_or_else(|| unsupported("source lacks ComObjectTable"))?;
    let mut spans = vec![object];
    for (name, placement, address_header) in [
        ("AddressTable", code.address_table.as_ref(), true),
        ("AssociationTable", code.association_table.as_ref(), false),
    ] {
        let placement = placement.ok_or_else(|| unsupported("source lacks a group table"))?;
        let (segment, start) = placed(code, placement, name)?;
        let count = placement
            .max_entries
            .ok_or_else(|| unsupported("table has no capacity"))? as usize;
        let length = count
            .checked_add(usize::from(address_header))
            .and_then(|count| count.checked_mul(2))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| unsupported("table capacity overflows"))?;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= segment.size as usize)
            .ok_or_else(|| unsupported("reserved table capacity exceeds source segment"))?;
        if spans
            .iter()
            .any(|(id, first, stop)| *id == segment.id && start < *stop && *first < end)
        {
            return Err(unsupported("reserved table regions overlap"));
        }
        spans.push((segment.id.clone(), start, end));
    }
    Ok(spans)
}
