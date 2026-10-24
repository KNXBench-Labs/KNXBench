//! Decodes only verified image placements; retains every compared raw memory range.

mod parameters;

use crate::image::DownloadImage;
use knx_core::commissioning::device_backup::DeviceBackup;
use knx_core::commissioning::memory_download::MemoryDownloadPlan;

/// A read-only presentation value; `None` is unknown, never an empty table or zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonRow {
    pub kind: &'static str,
    pub label: String,
    pub address: u32,
    pub project: Option<String>,
    pub device: Option<String>,
    pub reason: Option<String>,
    /// Exact captured field identity; never reconstructed from the label.
    /// Present even for an unknown parameter row, which remains non-adoptable.
    pub parameter: Option<crate::image::ParameterAttribution>,
    /// Typed per-side association entry; present on every `association` row.
    pub association: Option<AssociationAttribution>,
}

/// One association-table entry, resolved through its own side's address table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssociationEntry {
    /// 1-based position in that side's address table (§4.16.3.3.2).
    pub tsap: u8,
    /// The group object number, the ASAP.
    pub object: u8,
    /// The group address the TSAP selects in that side's address table.
    pub group_address: knx_core::GroupAddress,
}

/// What one side holds at an association slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationSlot {
    /// A completely decoded entry.
    Entry(AssociationEntry),
    /// The slot lies past this side's verified Current Size: no entry.
    Beyond,
}

/// Both sides of one association slot. `None` is unknown, never empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssociationAttribution {
    pub project: Option<AssociationSlot>,
    pub device: Option<AssociationSlot>,
}

/// Decode only the table format the image writer already verifies.
pub fn decode(
    image: &DownloadImage,
    held: &DeviceBackup,
    plan: &MemoryDownloadPlan,
) -> Vec<ComparisonRow> {
    use knx_core::commissioning::memory_download::MemoryDownloadStep;
    let device: std::collections::BTreeMap<u32, u8> = held
        .regions
        .iter()
        .flat_map(|r| {
            r.octets
                .iter()
                .enumerate()
                .map(move |(i, b)| (u32::from(r.address) + i as u32, *b))
        })
        .collect();
    let project: std::collections::BTreeMap<u32, u8> = plan
        .steps
        .iter()
        .filter_map(|step| match step {
            MemoryDownloadStep::WriteMemory { address, octets } => Some((*address, octets)),
            _ => None,
        })
        .flat_map(|(address, octets)| {
            octets
                .iter()
                .enumerate()
                .map(move |(i, b)| (u32::from(address) + i as u32, *b))
        })
        .collect();
    let mut rows = Vec::new();
    // group_tables' byte order and TSAP interpretation are verified for 0701 only.
    if held.mask.0 == 0x0701 && plan.mask.0 == 0x0701 {
        let address = table(image, image.code.address_table.as_ref(), true);
        if let Some(table) = address {
            let start = table.start;
            let pn = count(&project, table);
            let dn = count(&device, table);
            rows.push(row(
                "addressCount",
                "",
                start,
                pn.map(|n| n.to_string()),
                dn.map(|n| n.to_string()),
            ));
            for i in 0..pn.unwrap_or(0).max(dn.unwrap_or(0)) {
                let at = start + 1 + i * 2;
                let value = |data: &std::collections::BTreeMap<u32, u8>, n: Option<u32>| {
                    if i >= n? {
                        return None;
                    }
                    let raw = word(data, table, at)?;
                    Some(if i == 0 {
                        knx_core::IndividualAddress::from_raw(raw).to_string()
                    } else {
                        group_address(raw)
                    })
                };
                rows.push(row(
                    if i == 0 {
                        "individualAddress"
                    } else {
                        "groupAddress"
                    },
                    if i == 0 { String::new() } else { i.to_string() },
                    at,
                    value(&project, pn),
                    value(&device, dn),
                ));
            }
        }
        if let Some(table) = table(image, image.code.association_table.as_ref(), false) {
            let start = table.start;
            let pn = count(&project, table);
            let dn = count(&device, table);
            rows.push(row(
                "associationCount",
                "",
                start,
                pn.map(|n| n.to_string()),
                dn.map(|n| n.to_string()),
            ));
            for i in 0..pn.unwrap_or(0).max(dn.unwrap_or(0)) {
                let at = start + 1 + i * 2;
                let slot = |data: &std::collections::BTreeMap<u32, u8>, n: Option<u32>| {
                    if i >= n? {
                        return Some(AssociationSlot::Beyond);
                    }
                    let tsap = *data.get(&at)?;
                    let object = *data.get(&(at + 1))?;
                    let addresses = address?;
                    if tsap == 0
                        || u32::from(tsap) >= count(data, addresses)?
                        || at + 1 >= table.end
                    {
                        return None;
                    }
                    let raw = word(data, addresses, addresses.start + 1 + u32::from(tsap) * 2)?;
                    Some(AssociationSlot::Entry(AssociationEntry {
                        tsap,
                        object,
                        group_address: knx_core::GroupAddress::from_raw(raw),
                    }))
                };
                let text = |slot: Option<AssociationSlot>| match slot? {
                    AssociationSlot::Entry(entry) => Some(format!(
                        "TSAP {} → {} · object {}",
                        entry.tsap,
                        group_address(entry.group_address.raw()),
                        entry.object
                    )),
                    AssociationSlot::Beyond => None,
                };
                let attribution = AssociationAttribution {
                    project: slot(&project, pn),
                    device: slot(&device, dn),
                };
                let mut entry = row(
                    "association",
                    i.to_string(),
                    at,
                    text(attribution.project),
                    text(attribution.device),
                );
                entry.association = Some(attribution);
                rows.push(entry);
            }
        }
    }
    if held.mask == plan.mask && plan.mask.0 & 0xFFF0 == 0x0700 {
        rows.extend(parameters::rows(image, &project, &device));
    }
    // Keep *all* original ranges, not only changed/decoded fields. A decoder can
    // never turn bytes it cannot attribute into a silent loss (including padding).
    for region in &held.regions {
        let start = u32::from(region.address);
        let planned: Option<Vec<u8>> = (0..region.octets.len())
            .map(|i| project.get(&(start + i as u32)).copied())
            .collect();
        rows.push(row(
            "raw",
            "",
            start,
            planned.as_deref().map(hex),
            Some(hex(&region.octets)),
        ));
    }
    rows
}

fn row(
    kind: &'static str,
    label: impl Into<String>,
    address: u32,
    project: Option<String>,
    device: Option<String>,
) -> ComparisonRow {
    let reason = (project.is_none() || device.is_none()).then(|| {
        "Missing, invalid or outside the compared memory ranges; raw bytes are retained below"
            .to_string()
    });
    ComparisonRow {
        kind,
        label: label.into(),
        address,
        project,
        device,
        reason,
        parameter: None,
        association: None,
    }
}

#[derive(Clone, Copy)]
struct Table {
    start: u32,
    end: u32,
    capacity: u32,
    includes_individual: bool,
}

fn table(
    image: &DownloadImage,
    placement: Option<&crate::code::TablePlacement>,
    includes_individual: bool,
) -> Option<Table> {
    let placement = placement?;
    let segment = image.segment(placement.code_segment.as_deref()?)?;
    let offset = placement.offset?;
    let available = (segment.octets.len() as u32)
        .checked_sub(offset)?
        .checked_sub(1)?;
    let start = segment.address.checked_add(offset)?;
    let capacity = placement
        .max_entries?
        .saturating_add(u32::from(includes_individual))
        .min(available / 2)
        .min(255);
    let end = start.checked_add(1 + capacity * 2)?;
    if end > u32::from(u16::MAX) + 1 {
        return None;
    }
    Some(Table {
        start,
        end,
        capacity,
        includes_individual,
    })
}

fn count(data: &std::collections::BTreeMap<u32, u8>, table: Table) -> Option<u32> {
    let count = u32::from(*data.get(&table.start)?);
    (count <= table.capacity && (!table.includes_individual || count > 0)).then_some(count)
}

fn word(data: &std::collections::BTreeMap<u32, u8>, table: Table, address: u32) -> Option<u16> {
    if address < table.start || address.checked_add(1)? >= table.end {
        return None;
    }
    Some(u16::from_be_bytes([
        *data.get(&address)?,
        *data.get(&(address + 1))?,
    ]))
}

fn group_address(raw: u16) -> String {
    format!("{}/{}/{}", raw >> 11, (raw >> 8) & 7, raw & 255)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
