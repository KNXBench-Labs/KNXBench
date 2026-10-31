//! Prove exclusive inactive bytes without changing source images, values or byte permissions.
use super::*;
use crate::download_plan::InactiveWriteSelectionError as Refusal;
use crate::dynamic::{load_tree, ControlKind, Test, MAX_EVALUATION_WORK};

mod namespace;

/// Ordered source segment identity and per-byte selection (not a replacement source mask).
pub(crate) type Selection = Vec<(String, u32, Vec<bool>)>;

pub(crate) fn selection(
    conn: &Connection,
    request: &ImageRequest,
    image: &DownloadImage,
) -> Result<Selection, Refusal> {
    let types = ParameterTypes::load(conn, &request.program_id, &image.code)
        .map_err(|_| Refusal::UnresolvedOwnership)?;
    let current = permissions(conn, request, image, &types)?;
    let mut defaults = request.clone();
    defaults.values.clear();
    defaults.links.clear();
    defaults.flag_overrides.clear();
    let default_image =
        build_download_image(conn, &defaults).map_err(|_| Refusal::UnresolvedOwnership)?;
    let source = permissions(conn, &defaults, &default_image, &types)?;
    if current != source {
        return Err(Refusal::DifferentSourceDefaultTopology);
    }
    Ok(current)
}

fn permissions(
    conn: &Connection,
    request: &ImageRequest,
    image: &DownloadImage,
    types: &ParameterTypes,
) -> Result<Selection, Refusal> {
    let unresolved = || Refusal::UnresolvedOwnership;
    let tree = load_tree(conn, &request.program_id, "").map_err(|_| unresolved())?;
    let nodes: BTreeMap<_, _> = tree.structural_nodes().map(|n| (n.node_id, n)).collect();
    let mut permission: Selection = image
        .segments
        .iter()
        .map(|s| (s.id.clone(), s.address, vec![true; s.octets.len()]))
        .collect();
    // Only a ParameterRefRef below a `when` can become inactive. Without one nothing can be
    // omitted, so no provenance is needed and the regular plan stands unchanged.
    // A dangling or cyclic ancestry counts as conditional, so it still needs the full proof.
    let below_when = |mut parent: Option<i64>| {
        for _ in 0..=nodes.len() {
            let Some(id) = parent else { return false };
            let Some(node) = nodes.get(&id) else {
                return true;
            };
            if node.kind == "when" {
                return true;
            }
            parent = node.parent_id;
        }
        true
    };
    if !nodes
        .values()
        .any(|n| n.kind == "ParameterRefRef" && below_when(n.parent_id))
    {
        return Ok(permission);
    }
    namespace::prove(
        conn,
        &request.program_id,
        nodes.values().map(|n| n.kind.as_str()),
    )?;
    // `extra` has always retained uncaptured attributes. Its NULL/non-NULL
    // presence is lossless even for empty/newline/= values; do not interpret it.
    // A skipped branch is part of the proof, not an excuse to ignore its evidence.
    if tree.has_unmodelled_attributes() {
        return Err(unresolved());
    }
    let (values, activation) = activate(conn, request, types).map_err(|_| unresolved())?;
    if !activation.diagnostics.is_empty() {
        return Err(unresolved());
    }
    let mut children: BTreeMap<Option<i64>, Vec<i64>> = BTreeMap::new();
    for n in nodes.values() {
        children.entry(n.parent_id).or_default().push(n.node_id);
    }
    let mut stack: Vec<_> = children
        .get(&None)
        .into_iter()
        .flatten()
        .map(|id| (*id, false, 0))
        .collect();
    let mut visited = BTreeSet::new();
    let mut conditional_refs = BTreeSet::new();
    while let Some((id, conditional, depth)) = stack.pop() {
        if depth > 128 || visited.len() >= MAX_EVALUATION_WORK || !visited.insert(id) {
            return Err(unresolved());
        }
        let node = nodes.get(&id).ok_or_else(unresolved)?;
        let child_ids = children.get(&Some(id)).map(Vec::as_slice).unwrap_or(&[]);
        match node.kind.as_str() {
            "Dynamic"
            | "ChannelIndependentBlock"
            | "Channel"
            | "ParameterBlock"
            | "ParameterSeparator" => {}
            "choose" => {
                if node.control_kind != Some(ControlKind::Comparable)
                    || node.ref_id.is_none()
                    || child_ids.is_empty()
                    || child_ids.iter().any(|id| nodes[id].kind != "when")
                {
                    return Err(unresolved());
                }
                let defaults = child_ids.iter().filter(|id| nodes[id].is_default).count();
                if defaults > 1 {
                    return Err(unresolved());
                }
            }
            "when" => {
                if node
                    .parent_id
                    .and_then(|p| nodes.get(&p))
                    .is_none_or(|p| p.kind != "choose")
                    || (node.is_default && node.test.is_some())
                    || (!node.is_default
                        && node.test.as_deref().is_none_or(|t| Test::parse(t).is_err()))
                {
                    return Err(unresolved());
                }
            }
            "ParameterRefRef" => {
                if !child_ids.is_empty() {
                    return Err(unresolved());
                }
                if conditional {
                    conditional_refs.insert(node.ref_id.as_deref().ok_or_else(unresolved)?);
                }
            }
            "ComObjectRefRef" if child_ids.is_empty() => {}
            _ => return Err(unresolved()),
        }
        for child in child_ids {
            stack.push((*child, conditional || node.kind == "when", depth + 1));
        }
    }
    if visited.len() != nodes.len() {
        return Err(unresolved());
    }
    let active: BTreeSet<_> = activation
        .parameter_refs
        .iter()
        .map(|a| types.parameter_of(&a.ref_id).map(str::to_owned))
        .collect::<Result<_, _>>()
        .map_err(|_| unresolved())?;
    let mut excluded = BTreeSet::new();
    for reference in conditional_refs {
        let parameter = types.parameter_of(reference).map_err(|_| unresolved())?;
        if !active.contains(parameter) {
            excluded.insert(parameter.to_owned());
        }
    }
    if excluded.is_empty() {
        return Ok(permission);
    }
    if image.code.has_legacy_patch_always || image.code.has_download_invisible_parameters {
        return Err(unresolved());
    }
    // Resolve the complete placement inventory, not just the fields the image happened to overlay.
    let mut inactive: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut other: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for segment in &image.segments {
        inactive.insert(segment.id.clone(), vec![0; segment.octets.len()]);
        other.insert(segment.id.clone(), vec![0; segment.octets.len()]);
    }
    let mut references = BTreeMap::new();
    for (reference, facts) in &types.refs {
        references
            .entry(facts.parameter.as_str())
            .or_insert(reference.as_str());
    }
    let mut octet_owners: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    let mut work = visited.len();
    for (parameter, placement) in &image.code.parameters {
        let reference = *references.get(parameter.as_str()).ok_or_else(unresolved)?;
        let raw = values.get_unscoped(reference).ok_or_else(unresolved)?;
        types.value(reference, raw).map_err(|_| unresolved())?;
        let size = types.size(reference).map_err(|_| unresolved())?;
        let (segment, field) = field_of(parameter, placement, size).map_err(|_| unresolved())?;
        work = work
            .checked_add(size as usize)
            .filter(|w| *w <= MAX_EVALUATION_WORK)
            .ok_or_else(unresolved)?;
        let is_inactive = excluded.contains(parameter);
        if is_inactive && field.bit_offset == 0 && field.size_in_bit == 8 {
            octet_owners
                .entry(segment)
                .or_default()
                .insert(field.offset as usize);
        }
        let bits = if is_inactive {
            &mut inactive
        } else {
            &mut other
        };
        mark(bits, segment, field)?;
    }
    // Only actual winning overlays count as active: the existing image already resolved Unions.
    let mut winning: BTreeMap<String, Vec<u8>> = image
        .segments
        .iter()
        .map(|s| (s.id.clone(), vec![0; s.octets.len()]))
        .collect();
    for field in &image.parameter_fields {
        mark(&mut winning, &field.segment, field.field)?;
    }
    let object_span =
        com_object_span_with_layout(&image.code, ImageLayout::Easy3).map_err(|_| unresolved())?;
    for (segment_id, _, selected) in &mut permission {
        let segment = image.segment(segment_id).ok_or_else(unresolved)?;
        for (offset, selected) in selected.iter_mut().enumerate() {
            let i = inactive[segment_id][offset];
            if i == 0 || segment.mask.as_ref().is_some_and(|m| m[offset] != 0xFF) {
                continue;
            }
            let a = winning[segment_id][offset];
            let required = image
                .table_writes
                .iter()
                .any(|t| t.segment_id == *segment_id && t.start <= offset && offset < t.end)
                || object_span.as_ref().is_some_and(|(id, start, end)| {
                    id == segment_id && *start <= offset && offset < *end
                });
            if a == 0xFF || required {
                continue;
            }
            if a != 0 {
                return Err(Refusal::SharedActiveInactiveByte);
            }
            if i != 0xFF || other[segment_id][offset] != 0 {
                return Err(unresolved());
            }
            // The first admitted omission is an aligned eight-bit field, not a multibyte inference.
            let has_octet_owner = octet_owners
                .get(segment_id.as_str())
                .is_some_and(|owners| owners.contains(&offset));
            if !has_octet_owner {
                return Err(unresolved());
            }
            *selected = false;
        }
    }
    Ok(permission)
}

fn mark(
    bits: &mut BTreeMap<String, Vec<u8>>,
    segment: &str,
    field: ParameterField,
) -> Result<(), Refusal> {
    let octets = bits.get_mut(segment).ok_or(Refusal::UnresolvedOwnership)?;
    let (start, end) = bit_span(field);
    if end > octets.len() as u64 * 8 {
        return Err(Refusal::UnresolvedOwnership);
    }
    for bit in start..end {
        octets[bit as usize / 8] |= 1 << (7 - bit % 8);
    }
    Ok(())
}
