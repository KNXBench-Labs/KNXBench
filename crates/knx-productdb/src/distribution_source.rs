//! Resolves compact format-2 provenance and counts canonical JSON without allocating output.

use super::*;
use std::io::{self, Write};

/// Format-2 source reference: JSON tuple [member ordinal or null, one-based location ordinal].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(Option<u32>, u32)", into = "(Option<u32>, u32)")]
pub struct DistributionSourceRef {
    pub member_ordinal: Option<u32>,
    pub location: u32,
}
impl From<(Option<u32>, u32)> for DistributionSourceRef {
    fn from((member_ordinal, location): (Option<u32>, u32)) -> Self {
        Self {
            member_ordinal,
            location,
        }
    }
}
impl From<DistributionSourceRef> for (Option<u32>, u32) {
    fn from(r: DistributionSourceRef) -> Self {
        (r.member_ordinal, r.location)
    }
}
/// Interned expanded and original element names; prefixes remain observable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSourceName {
    pub namespace: String,
    pub local: String,
    pub raw: String,
}
/// One occurrence, never a full path. Parent/name indices are zero-based.
/// Text entries point to their element owner and keep the original global token ordinal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionLocation {
    pub parent: Option<u32>,
    pub name: u32,
    pub sibling: u32,
    pub start: u64,
    pub end: u64,
    pub text_token: Option<u64>,
}

impl DistributionInspection {
    /// Resolves untrusted/deserialized references without recursion or unchecked indexing.
    /// Rejects ambiguous documents, cycles, invalid names/ranges/parents and excessive depth.
    pub fn resolve_coordinate(
        &self,
        source: DistributionSourceRef,
    ) -> Result<DistributionCoordinate> {
        let invalid = || DistributionInspectionError::Reader("invalid source map reference".into());
        let mut documents = self
            .documents
            .iter()
            .filter(|d| d.member_ordinal == source.member_ordinal);
        let doc = documents.next().ok_or_else(invalid)?;
        if documents.next().is_some() {
            return Err(invalid());
        }
        let index = source.location.checked_sub(1).ok_or_else(invalid)? as usize;
        let occurrence = doc.locations.get(index).ok_or_else(invalid)?;
        let mut chain = Vec::new();
        let mut current = index;
        let mut text = None;
        loop {
            if chain.len() >= DistributionLimits::default().max_depth || chain.contains(&current) {
                return Err(invalid());
            }
            let location = doc.locations.get(current).ok_or_else(invalid)?;
            if location.start > location.end || location.end > doc.size || location.sibling == 0 {
                return Err(invalid());
            }
            let name = doc.names.get(location.name as usize).ok_or_else(invalid)?;
            if name.local.is_empty()
                || name.local.len() > 512
                || name.local.contains(['/', '[', ']'])
            {
                return Err(invalid());
            }
            if let Some(token) = location.text_token {
                if current != index || token == 0 {
                    return Err(invalid());
                }
                text = Some(token);
            } else {
                chain.push(current);
            }
            match location.parent {
                Some(parent) => {
                    let parent_index = parent as usize;
                    // Generated maps are preorder; this also makes cycles and self-links impossible.
                    if parent_index >= current {
                        return Err(invalid());
                    }
                    let owner = doc.locations.get(parent_index).ok_or_else(invalid)?;
                    if owner.text_token.is_some()
                        || owner.start > location.start
                        || owner.end < location.end
                    {
                        return Err(invalid());
                    }
                    current = parent_index;
                }
                None => {
                    if current != 0 || location.text_token.is_some() {
                        return Err(invalid());
                    }
                    break;
                }
            }
        }
        let element_ordinal = chain.first().copied().ok_or_else(invalid)? as u64 + 1;
        let mut path = String::new();
        for i in chain.into_iter().rev() {
            let l = &doc.locations[i];
            use std::fmt::Write;
            write!(path, "/{}[{}]", doc.names[l.name as usize].local, l.sibling)
                .map_err(|_| invalid())?;
        }
        if let Some(token) = text {
            use std::fmt::Write;
            write!(path, "/text()[{token}]").map_err(|_| invalid())?;
        }
        Ok(DistributionCoordinate {
            member_ordinal: source.member_ordinal,
            element_ordinal,
            start_byte: occurrence.start,
            end_byte: occurrence.end,
            path,
        })
    }

    /// Exact compact serde_json representation size (not heap/RSS, not pretty-printed JSON).
    pub fn serialized_size(&self) -> Result<usize> {
        json_size(self, usize::MAX)
    }
}

struct Counter {
    bytes: usize,
    cap: usize,
}
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("serialized report bytes"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(super) fn json_size(value: &impl Serialize, cap: usize) -> Result<usize> {
    let mut counter = Counter { bytes: 0, cap };
    serde_json::to_writer(&mut counter, value).map_err(|_| limit("serialized report bytes"))?;
    Ok(counter.bytes)
}

/// Appends rows while keeping `total` the exact compact-JSON size: each row plus a separating comma.
pub(super) fn append_counted<T: Serialize>(
    dst: &mut Vec<T>,
    rows: Vec<T>,
    total: &mut usize,
    cap: usize,
) -> Result<()> {
    dst.reserve(rows.len());
    for row in rows {
        let grown = json_size(&row, cap)?
            .checked_add(usize::from(!dst.is_empty()))
            .and_then(|n| total.checked_add(n))
            .filter(|n| *n <= cap)
            .ok_or_else(|| limit("serialized report bytes"))?;
        *total = grown;
        dst.push(row);
    }
    Ok(())
}
