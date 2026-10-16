//! System-1 source locations are checked without synthesizing pointers or procedure steps.

use super::{legacy_source, placed, ImageError, ImageLayout};
use crate::code::ProgramCode;

/// Require source-backed Type1 locations and protect the two pointer resources.
///
/// Resources v01.10.01 pp223/246/265: address table at0116h; association
/// and object tables use the source octet at0111h/0112h plus0100h.
/// This bounded entry admits only0012/DefaultProcedure sources, not execution.
pub(super) fn table_spans(code: &ProgramCode) -> Result<Vec<(String, usize, usize)>, ImageError> {
    let unsupported = |what: &str| ImageError::Unsupported {
        what: format!("System1 {what}"),
    };
    if code.mask_version.as_deref() != Some("MV-0012")
        || code.load_procedure_style.as_deref() != Some("DefaultProcedure")
    {
        return Err(unsupported("requires explicit0012/DefaultProcedure source"));
    }
    if code.segments.iter().any(|s| s.data.is_none()) {
        return Err(unsupported("source segment ships no Data"));
    }

    let mut spans = legacy_source::table_spans(code, ImageLayout::System1)?;
    let objects = code
        .com_object_table
        .as_ref()
        .ok_or_else(|| unsupported("source lacks ComObjectTable"))?;
    let (segment, start) = placed(code, objects, "ComObjectTable")?;
    let data = segment
        .data
        .as_ref()
        .expect("validated complete source bytes");
    let count = usize::from(data[start]); // Shared span validation checked length.
    for descriptor in data[start + 2..start + 2 + count * 3].as_chunks::<3>().0 {
        if descriptor[1] & 0x80 == 0 || descriptor[2] > 14 {
            return Err(unsupported(
                "source Type1 descriptor has invalid reserved/type fields",
            ));
        }
    }
    let addresses = code
        .address_table
        .as_ref()
        .ok_or_else(|| unsupported("source lacks AddressTable"))?;
    let (segment, offset) = placed(code, addresses, "AddressTable")?;
    if u64::from(segment.address) + offset as u64 != 0x0116 {
        return Err(unsupported("address table must be at0116h"));
    }
    for (placement, name, pointer_address) in [
        (code.association_table.as_ref(), "AssociationTable", 0x0111),
        (code.com_object_table.as_ref(), "ComObjectTable", 0x0112),
    ] {
        let placement = placement.ok_or_else(|| unsupported("source lacks table placement"))?;
        let (target, offset) = placed(code, placement, name)?;
        let source = code
            .segments
            .iter()
            .find(|s| {
                s.address <= pointer_address && pointer_address < s.address.saturating_add(s.size)
            })
            .ok_or_else(|| unsupported("source lacks table pointer octet"))?;
        let at = (pointer_address - source.address) as usize;
        let pointer = source
            .data
            .as_ref()
            .and_then(|data| data.get(at))
            .ok_or_else(|| unsupported("source table pointer is out of bounds"))?;
        if !(0x19..=0xFE).contains(pointer)
            || u64::from(target.address) + offset as u64 != 0x0100 + u64::from(*pointer)
        {
            return Err(unsupported(
                "table placement disagrees with retained source pointer",
            ));
        }
        if spans
            .iter()
            .any(|(id, start, end)| *id == source.id && *start <= at && at < *end)
        {
            return Err(unsupported("table overlaps pointer resource"));
        }
        spans.push((source.id.clone(), at, at + 1));
    }
    Ok(spans)
}
