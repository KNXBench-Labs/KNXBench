//! The element dispatch for schema ≥21's `0.xml`: the same small state
//! machine as [`super::installation::parse_installation`] (schema 11), kept
//! as a textually separate sibling per ADR-0014 rather than shared, since the
//! two schemas' content models diverge enough (`Segment` interposed between
//! `Line` and `DeviceInstance`, `Locations`/`Space` replacing
//! `Buildings`/`BuildingPart`, flat `Links`/`ChannelId` replacing
//! `Connectors`, `ModuleInstances`/`GroupObjectTree` appearing at all) that a
//! shared abstraction would cost more than it saves.
//!
//! Two stacks are threaded through the loop: `path_stack`, the absolute
//! element path, and `frames`, the in-progress struct values waiting for
//! their children to finish. Wrapper elements push `path_stack` but never
//! `frames`. A third kind, [`Kind::RawCapture`], behaves like a wrapper for
//! walking purposes (children are still parsed normally) but additionally
//! remembers the byte offset its element opened at, so the matching close
//! event can slice `bytes[start..end]` and retain the whole subtree verbatim
//! for export — used for `ModuleInstances` and `GroupObjectTree`, both of
//! which need their children *understood* (to populate
//! `SourceDevice::module_instances`/`group_object_tree`) and their raw bytes
//! *retained* (since `map.rs`/export reconstructs neither element from the
//! modeled fields alone — see the plan's Global Constraints).
//!
//! `GroupRange`, `BuildingPart` and `Space` nest inside themselves (the
//! latter two share one collapse rule, matching how ETS's location
//! hierarchy — schema 11's `Buildings`/`BuildingPart`, schema ≥21's
//! `Locations`/`Space` — can be arbitrarily deep even though the reference
//! project measured here is flat). The known-element table registers one
//! path per element, so a nested occurrence's absolute path is collapsed to
//! the same path as the outer one before it is looked up; the *uncollapsed*
//! path is still what gets reported in `UnknownConstruct`/`RetainedAttribute`
//! xpaths.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::known::KnownSchema;
use crate::source::{
    RetainedAttribute, RetainedElement, SourceArea, SourceArgument, SourceBinaryDataRef,
    SourceBuildingPart, SourceComObjectInstance, SourceDevice, SourceDocument, SourceGroupAddress,
    SourceGroupRange, SourceInstallation, SourceLine, SourceModuleInstance,
    SourceParameterInstance,
};

use super::{attr_map, skip_and_capture, ParseError, ParseOutput, UnknownAggregator, UnknownKind};

/// One struct being built, waiting for its children to close before it is
/// converted and attached to its parent.
enum Frame {
    Installation(SourceInstallation),
    Area(SourceArea),
    Line(SourceLine),
    // Boxed: `SourceDevice` is far larger than every other variant (clippy
    // large_enum_variant), and a `Frame` is short-lived scaffolding anyway
    // — the indirection costs nothing a caller notices.
    Device(Box<SourceDevice>),
    ComObject(SourceComObjectInstance),
    Parameter(SourceParameterInstance),
    BuildingPart(SourceBuildingPart),
    GroupRange(SourceGroupRange),
    GroupAddress(SourceGroupAddress),
    BinaryDataRef(SourceBinaryDataRef),
    ModuleInstance(SourceModuleInstance),
    Argument(SourceArgument),
}

/// Whether an element that went through the normal open/close cycle pushed a
/// [`Frame`], was a pure wrapper, or is a wrapper that additionally needs its
/// raw bytes sliced out at close time (see the module doc comment).
enum Kind {
    Wrapper,
    Frame,
    /// Carries the byte offset the element opened at and its absolute xpath
    /// (both computed once, at open time) so the matching `End` event can
    /// slice `bytes[start..reader.buffer_position()]` without recomputing
    /// either.
    RawCapture(u64, String),
}

/// Collects raw `(name, value)` attribute pairs not yet classified against
/// the known-element table, and lets modeled fields be pulled out by name;
/// whatever is left after every modeled field has been taken is either a
/// known-but-unmapped attribute or, for elements this call already flagged
/// as fully unknown, never reaches here at all.
struct AttrBag(Vec<(String, String)>);

impl AttrBag {
    fn take(&mut self, name: &str) -> Option<String> {
        let idx = self.0.iter().position(|(k, _)| k == name)?;
        Some(self.0.remove(idx).1)
    }

    fn require(&mut self, name: &str, xpath: &str) -> Result<String, ParseError> {
        self.take(name)
            .ok_or_else(|| ParseError::MissingRequiredAttribute {
                xpath: xpath.to_string(),
                name: name.to_string(),
            })
    }

    /// Whatever remains once every modeled field has been taken: attributes
    /// the table knows about but this struct has no dedicated field for.
    /// Retained, not dropped, but not reported as unknown — the table does
    /// list them.
    fn into_retained(self, xpath: &str) -> Vec<RetainedAttribute> {
        self.0
            .into_iter()
            .map(|(name, value)| RetainedAttribute {
                xpath: xpath.to_string(),
                name,
                value,
            })
            .collect()
    }
}

/// Splits an element's raw attribute pairs into the known ones (for
/// `AttrBag::take`) and the unknown ones — each unknown one is both recorded
/// in `aggregator` (for the report) and turned into a `RetainedAttribute`
/// (for the owning struct's `other` field), per CLAUDE.md: the fact and the
/// value both survive.
fn partition_attrs(
    pairs: Vec<(String, String)>,
    known_names: &[&str],
    xpath: &str,
    aggregator: &mut UnknownAggregator,
) -> (AttrBag, Vec<RetainedAttribute>) {
    let mut known = Vec::with_capacity(pairs.len());
    let mut unknown = Vec::new();
    for (name, value) in pairs {
        if known_names.contains(&name.as_str()) {
            known.push((name, value));
        } else {
            aggregator.record(xpath, UnknownKind::Attribute, &name, Some(value.clone()));
            unknown.push(RetainedAttribute {
                xpath: xpath.to_string(),
                name,
                value,
            });
        }
    }
    (AttrBag(known), unknown)
}

fn known_attributes<'s>(schema: &'s KnownSchema, path: &str) -> &'s [&'s str] {
    schema
        .elements
        .iter()
        .find(|e| e.path == path)
        .map(|e| e.attributes)
        .unwrap_or(&[])
}

/// Elements that genuinely nest inside themselves at any depth, sharing one
/// known-element table entry regardless of how deep. `BuildingPart` and
/// `Space` are two different element *names* for the same location-hierarchy
/// concept (schema 11 vs schema ≥21) but each still only ever nests under
/// its own name, so both are listed; collapsing only ever matches an
/// element against a consecutive occurrence of the *same* name.
const SELF_RECURSIVE: &[&str] = &["GroupRange", "BuildingPart", "Space"];

/// Collapses consecutive duplicate segments for [`SELF_RECURSIVE`] elements
/// so a recursive element's path matches the single entry the
/// known-element table registers for it, however deep the nesting.
fn collapsed(stack: &[String]) -> String {
    let mut out: Vec<&str> = Vec::with_capacity(stack.len());
    for seg in stack {
        if out.last() == Some(&seg.as_str()) && SELF_RECURSIVE.contains(&seg.as_str()) {
            continue;
        }
        out.push(seg);
    }
    format!("/{}", out.join("/"))
}

fn real_path(stack: &[String]) -> String {
    format!("/{}", stack.join("/"))
}

/// Parses one schema-≥21 `0.xml` document (an installation's topology,
/// devices and group addresses) into a [`SourceDocument`], tolerantly: an
/// element or attribute the known-element table for `schema` does not list
/// is retained and reported, never a fatal error on its own. Only malformed
/// XML and a missing required identity attribute (`Id`/`RefId` on a modeled
/// element) fail the whole parse.
pub fn parse_installation_v21(
    bytes: &[u8],
    source_path: &str,
    schema: &KnownSchema,
) -> Result<ParseOutput, ParseError> {
    // See `parse_installation`'s identical stripping for why: `Reader`
    // silently absorbs a leading UTF-8 BOM into its own position bookkeeping,
    // so every `bytes[pos_before..end]` raw-capture slice must be indexed
    // against a `bytes` view that has already had the same three bytes
    // removed, or every capture lands three bytes early. The reference
    // project measured here (`KV v2.5 - demo.knxproj`) carries this BOM too.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut reader = Reader::from_reader(bytes);

    let mut document = SourceDocument {
        schema_version: schema.version,
        ..Default::default()
    };
    let mut path_stack: Vec<String> = Vec::new();
    let mut kind_stack: Vec<Kind> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut aggregator = UnknownAggregator::default();
    let mut retained_elements: Vec<RetainedElement> = Vec::new();

    loop {
        let pos_before = reader.buffer_position();
        let event = reader.read_event().map_err(|e| ParseError::Xml {
            source_path: source_path.to_string(),
            position: reader.buffer_position(),
            cause: e.to_string(),
        })?;

        match event {
            Event::Eof => break,

            Event::Start(start) => {
                open_element(
                    &start,
                    false,
                    &mut reader,
                    bytes,
                    pos_before,
                    source_path,
                    schema,
                    &mut path_stack,
                    &mut kind_stack,
                    &mut frames,
                    &mut document,
                    &mut aggregator,
                    &mut retained_elements,
                )?;
            }

            Event::Empty(start) => {
                open_element(
                    &start,
                    true,
                    &mut reader,
                    bytes,
                    pos_before,
                    source_path,
                    schema,
                    &mut path_stack,
                    &mut kind_stack,
                    &mut frames,
                    &mut document,
                    &mut aggregator,
                    &mut retained_elements,
                )?;
            }

            Event::End(_) => {
                let local = path_stack.pop().expect("End without matching Start");
                let kind = kind_stack
                    .pop()
                    .expect("kind stack out of sync with path stack");
                match kind {
                    Kind::Frame => {
                        let parent_name = path_stack.last().map(String::as_str).unwrap_or("");
                        let closed = frames
                            .pop()
                            .expect("frame stack out of sync with path stack");
                        attach_frame(&mut frames, &mut document, parent_name, &local, closed);
                    }
                    Kind::RawCapture(start_pos, xpath) => {
                        let end = reader.buffer_position();
                        let raw = bytes[start_pos as usize..end as usize].to_vec();
                        let retained = RetainedElement {
                            xpath,
                            name: local.clone(),
                            raw,
                        };
                        if let Some(Frame::Device(d)) = frames.last_mut() {
                            match local.as_str() {
                                "ModuleInstances" => d.module_instances_raw = Some(retained),
                                "GroupObjectTree" => d.group_object_tree_raw = Some(retained),
                                _ => unreachable!(
                                    "Kind::RawCapture used for unexpected element {local}"
                                ),
                            }
                        }
                    }
                    Kind::Wrapper => {}
                }
            }

            _ => {} // Text, Comment, PI, Decl, CData, DocType: not structural, ignored.
        }
    }

    if !path_stack.is_empty() {
        return Err(ParseError::Xml {
            source_path: source_path.to_string(),
            position: reader.buffer_position(),
            cause: format!(
                "unexpected end of document inside <{}>",
                path_stack.join("/")
            ),
        });
    }

    Ok(ParseOutput {
        document,
        unknown: aggregator.into_sorted_vec(source_path),
        retained_elements,
    })
}

/// Handles one `Start` or `Empty` event: classifies the element by its
/// absolute path against `schema`, then either recurses into the normal
/// open/close cycle (wrapper or frame-bearing elements push `path_stack`
/// and, for frames, `frames`) or resolves the element on the spot (leaf
/// string references, the verbatim-retained `BusAccess`/`ModuleInstances`/
/// `GroupObjectTree`, and anything the table does not know at all).
#[allow(clippy::too_many_arguments)]
fn open_element<'a>(
    start: &BytesStart<'a>,
    is_empty: bool,
    reader: &mut Reader<&'a [u8]>,
    bytes: &'a [u8],
    pos_before: u64,
    source_path: &str,
    schema: &KnownSchema,
    path_stack: &mut Vec<String>,
    kind_stack: &mut Vec<Kind>,
    frames: &mut Vec<Frame>,
    document: &mut SourceDocument,
    aggregator: &mut UnknownAggregator,
    retained_elements: &mut Vec<RetainedElement>,
) -> Result<(), ParseError> {
    let local = start.name().local_name().as_ref().to_string();
    let parent_name = path_stack.last().cloned().unwrap_or_default();

    path_stack.push(local.clone());
    let xpath = real_path(path_stack);
    let matching_path = collapsed(path_stack);
    path_stack.pop();

    let known = known_attributes(schema, &matching_path);
    let is_known_path = schema.elements.iter().any(|e| e.path == matching_path);

    // `KNX` and `Project` carry document-level identity, not a `Frame`.
    if local == "KNX" {
        let attrs = attr_map(start, source_path, pos_before)?;
        let (mut bag, unknown) = partition_attrs(attrs, known, &xpath, aggregator);
        document.created_by = bag.take("CreatedBy");
        document.tool_version = bag.take("ToolVersion");
        drop(bag); // no `other` bucket at document level; unknowns already reported
        drop(unknown);
        path_stack.push(local);
        kind_stack.push(Kind::Wrapper);
        return Ok(());
    }
    if local == "Project" {
        let attrs = attr_map(start, source_path, pos_before)?;
        let (mut bag, _) = partition_attrs(attrs, known, &xpath, aggregator);
        document.project_id = bag.require("Id", &xpath)?;
        path_stack.push(local);
        kind_stack.push(Kind::Wrapper);
        return Ok(());
    }

    // Deliberately-not-modeled but known: retained verbatim, not reported as
    // unknown, and never pushed onto either stack — the whole subtree is
    // consumed here.
    if local == "BusAccess" {
        let raw = if is_empty {
            bytes[pos_before as usize..reader.buffer_position() as usize].to_vec()
        } else {
            skip_and_capture(reader, bytes, start, pos_before, source_path)?
        };
        let retained = RetainedElement {
            xpath: xpath.clone(),
            name: local,
            raw,
        };
        if let Some(Frame::Line(line)) = frames.last_mut() {
            line.bus_access = Some(retained);
        }
        return Ok(());
    }

    // `Security` (schema ≥21, per-device sequence-number/timestamp bookkeeping
    // — RESEARCH has not investigated its semantics beyond the attribute
    // names): known but deliberately not modeled, retained verbatim exactly
    // like `BusAccess` above — a leaf whose whole subtree is consumed here,
    // never pushed onto either stack, attached straight onto the enclosing
    // `Frame::Device` (schema ≥21's `Security` sits directly under
    // `DeviceInstance`, so the device frame is always what's on top).
    if local == "Security" {
        let raw = if is_empty {
            bytes[pos_before as usize..reader.buffer_position() as usize].to_vec()
        } else {
            skip_and_capture(reader, bytes, start, pos_before, source_path)?
        };
        let retained = RetainedElement {
            xpath: xpath.clone(),
            name: local,
            raw,
        };
        if let Some(Frame::Device(d)) = frames.last_mut() {
            d.security_raw = Some(retained);
        }
        return Ok(());
    }

    // Genuinely unknown: retained verbatim and reported, subtree consumed
    // here, never pushed onto either stack.
    if !is_known_path {
        aggregator.record(&xpath, UnknownKind::Element, &local, None);
        let raw = if is_empty {
            bytes[pos_before as usize..reader.buffer_position() as usize].to_vec()
        } else {
            skip_and_capture(reader, bytes, start, pos_before, source_path)?
        };
        retained_elements.push(RetainedElement {
            xpath,
            name: local,
            raw,
        });
        return Ok(());
    }

    // Leaf string references: a single attribute, no children of their own,
    // attached straight into the enclosing frame.
    if matches!(local.as_str(), "Send" | "Receive" | "DeviceInstanceRef") {
        let attrs = attr_map(start, source_path, pos_before)?;
        let (mut bag, _) = partition_attrs(attrs, known, &xpath, aggregator);
        let attr_name = if local == "DeviceInstanceRef" {
            "RefId"
        } else {
            "GroupAddressRefId"
        };
        let value = bag.require(attr_name, &xpath)?;
        if !is_empty {
            reader
                .read_to_end(start.to_end().name())
                .map_err(|e| ParseError::Xml {
                    source_path: source_path.to_string(),
                    position: reader.buffer_position(),
                    cause: e.to_string(),
                })?;
        }
        attach_leaf_string(frames, &parent_name, &local, value);
        return Ok(());
    }

    // `Node`: a leaf, but unlike the leaf strings above it does not attach a
    // single value — it unions its `GroupObjectInstances` ids into the
    // enclosing device's authoritative object-id list (ADR-0014). `Type`/
    // `RefId` are known but deliberately not modeled at this level: the raw
    // `GroupObjectTree` capture (see below) is what preserves them for
    // export, so nothing here is actually lost, only left unstructured.
    if local == "Node" {
        let attrs = attr_map(start, source_path, pos_before)?;
        let (mut bag, _) = partition_attrs(attrs, known, &xpath, aggregator);
        if let Some(ids) = bag.take("GroupObjectInstances") {
            if let Some(Frame::Device(d)) = frames.last_mut() {
                for id in ids.split_whitespace() {
                    if !d.group_object_tree.iter().any(|x| x == id) {
                        d.group_object_tree.push(id.to_string());
                    }
                }
            }
        }
        if !is_empty {
            reader
                .read_to_end(start.to_end().name())
                .map_err(|e| ParseError::Xml {
                    source_path: source_path.to_string(),
                    position: reader.buffer_position(),
                    cause: e.to_string(),
                })?;
        }
        return Ok(());
    }

    // `Segment` is a transparent merge-up: schema ≥21 interposes it between
    // `Line` and `DeviceInstance`, and moves the medium/domain-address
    // attributes that schema 11 puts directly on `Line` down onto `Segment`
    // instead — but nothing downstream (`map.rs`, `knx-core`) has a
    // "segment" concept, so its attributes are folded straight into the
    // enclosing `Line` frame and its `DeviceInstance` children attach to
    // `Line` too (see the `"Segment"` arm added to `attach_frame` below).
    if local == "Segment" {
        let attrs = attr_map(start, source_path, pos_before)?;
        let (mut bag, _unknown) = partition_attrs(attrs, known, &xpath, aggregator);
        if let Some(Frame::Line(line)) = frames.last_mut() {
            if let Some(v) = bag.take("MediumTypeRefId") {
                if line.medium_type_ref_id.is_none() {
                    line.medium_type_ref_id = Some(v);
                } else {
                    line.other.push(RetainedAttribute {
                        xpath: xpath.clone(),
                        name: "MediumTypeRefId".into(),
                        value: v,
                    });
                }
            }
            if let Some(v) = bag.take("DomainAddress") {
                line.domain_address.get_or_insert(v);
            }
            if let Some(v) = bag.take("DomainAddressIsChecked") {
                line.domain_address_is_checked.get_or_insert(v);
            }
            if let Some(v) = bag.take("IPRoutingMulticastAddress") {
                line.ip_routing_multicast_address.get_or_insert(v);
            }
            if let Some(v) = bag.take("MulticastTTL") {
                line.multicast_ttl.get_or_insert(v);
            }
            line.other.extend(bag.into_retained(&xpath)); // Id, Number, Puid, leftovers
        }
        if is_empty {
            return Ok(()); // no DeviceInstance children to wait for
        }
        path_stack.push(local);
        kind_stack.push(Kind::Wrapper); // DeviceInstance children attach to Line, not Segment
        return Ok(());
    }

    // `ModuleInstances`: walked (its `ModuleInstance` children are modeled)
    // *and* retained raw for export in one pass — see the module doc
    // comment for why `Kind::RawCapture` exists.
    if local == "ModuleInstances" {
        if is_empty {
            let raw = bytes[pos_before as usize..reader.buffer_position() as usize].to_vec();
            if let Some(Frame::Device(d)) = frames.last_mut() {
                d.module_instances_raw = Some(RetainedElement {
                    xpath: xpath.clone(),
                    name: local.clone(),
                    raw,
                });
            }
            return Ok(());
        }
        path_stack.push(local);
        kind_stack.push(Kind::RawCapture(pos_before, xpath));
        return Ok(());
    }

    // `GroupObjectTree`: schema 21 nests `Nodes/Node` children (walked and
    // unioned by the `Node` case above); schema 23 instead carries a flat
    // `GroupObjectInstances` attribute directly here (no children at all).
    // Either way the whole subtree is also retained raw for export.
    if local == "GroupObjectTree" {
        let attrs = attr_map(start, source_path, pos_before)?;
        let (mut bag, _) = partition_attrs(attrs, known, &xpath, aggregator);
        if let Some(ids) = bag.take("GroupObjectInstances") {
            // schema 23's flat shape only
            if let Some(Frame::Device(d)) = frames.last_mut() {
                d.group_object_tree = ids.split_whitespace().map(str::to_string).collect();
            }
        }
        if is_empty {
            let raw = bytes[pos_before as usize..reader.buffer_position() as usize].to_vec();
            if let Some(Frame::Device(d)) = frames.last_mut() {
                d.group_object_tree_raw = Some(RetainedElement {
                    xpath: xpath.clone(),
                    name: local.clone(),
                    raw,
                });
            }
            return Ok(());
        }
        path_stack.push(local);
        kind_stack.push(Kind::RawCapture(pos_before, xpath));
        return Ok(());
    }

    // A wrapper: known, carries no data of its own, only structure.
    if is_wrapper(&local, &parent_name) {
        let attrs = attr_map(start, source_path, pos_before)?;
        partition_attrs(attrs, known, &xpath, aggregator); // any attrs here are unknown; reported, not retained anywhere
        if is_empty {
            return Ok(()); // an empty wrapper has no children to wait for
        }
        path_stack.push(local);
        kind_stack.push(Kind::Wrapper);
        return Ok(());
    }

    // Everything else the table knows about is frame-bearing.
    let attrs = attr_map(start, source_path, pos_before)?;
    let frame = build_frame(&local, &xpath, attrs, known, aggregator)?;
    if is_empty {
        attach_frame(frames, document, &parent_name, &local, frame);
    } else {
        path_stack.push(local);
        kind_stack.push(Kind::Frame);
        frames.push(frame);
    }
    Ok(())
}

fn is_wrapper(local: &str, parent_name: &str) -> bool {
    match local {
        "Installations"
        | "Topology"
        | "ParameterInstanceRefs"
        | "ComObjectInstanceRefs"
        | "Connectors"
        | "UnassignedDevices"
        | "Buildings"
        | "Locations"
        | "GroupAddresses"
        | "GroupRanges"
        | "Arguments"
        | "Nodes" => true,
        // The wrapper form of `BinaryData`; the leaf form's parent is `BinaryData` itself.
        "BinaryData" => parent_name == "DeviceInstance",
        _ => false,
    }
}

fn attach_leaf_string(frames: &mut [Frame], parent_name: &str, local: &str, value: String) {
    match (parent_name, local) {
        ("Connectors", "Send") => match frames.last_mut() {
            Some(Frame::ComObject(c)) => c.sends.push(value),
            _ => unreachable!("Send outside a ComObjectInstanceRef frame"),
        },
        ("Connectors", "Receive") => match frames.last_mut() {
            Some(Frame::ComObject(c)) => c.receives.push(value),
            _ => unreachable!("Receive outside a ComObjectInstanceRef frame"),
        },
        // `Space` is schema ≥21's spelling of `BuildingPart` (module doc);
        // both carry the same device-to-room assignment.
        ("BuildingPart" | "Space", "DeviceInstanceRef") => match frames.last_mut() {
            Some(Frame::BuildingPart(b)) => b.device_refs.push(value),
            _ => unreachable!("DeviceInstanceRef outside a BuildingPart frame"),
        },
        _ => unreachable!("unexpected leaf string {local} under {parent_name}"),
    }
}

fn build_frame(
    local: &str,
    xpath: &str,
    attrs: Vec<(String, String)>,
    known: &[&str],
    aggregator: &mut UnknownAggregator,
) -> Result<Frame, ParseError> {
    let (mut bag, unknown) = partition_attrs(attrs, known, xpath, aggregator);
    let frame = match local {
        "Installation" => Frame::Installation(SourceInstallation {
            installation_id: bag.take("InstallationId"),
            name: bag.take("Name"),
            default_line: bag.take("DefaultLine"),
            ip_routing_multicast_address: bag.take("IPRoutingMulticastAddress"),
            completion_status: bag.take("CompletionStatus"),
            areas: Vec::new(),
            unassigned_devices: Vec::new(),
            buildings: Vec::new(),
            group_ranges: Vec::new(),
            other: Vec::new(),
        }),
        "Area" => Frame::Area(SourceArea {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            address: bag.take("Address"),
            completion_status: bag.take("CompletionStatus"),
            lines: Vec::new(),
            other: Vec::new(),
        }),
        "Line" => Frame::Line(SourceLine {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            address: bag.take("Address"),
            medium_type_ref_id: bag.take("MediumTypeRefId"),
            domain_address: bag.take("DomainAddress"),
            domain_address_is_checked: bag.take("DomainAddressIsChecked"),
            ip_routing_multicast_address: bag.take("IPRoutingMulticastAddress"),
            multicast_ttl: bag.take("MulticastTTL"),
            completion_status: bag.take("CompletionStatus"),
            devices: Vec::new(),
            bus_access: None,
            other: Vec::new(),
        }),
        "DeviceInstance" => Frame::Device(Box::new(SourceDevice {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            description: bag.take("Description"),
            address: bag.take("Address"),
            product_ref_id: bag.take("ProductRefId"),
            hardware2program_ref_id: bag.take("Hardware2ProgramRefId"),
            last_modified: bag.take("LastModified"),
            last_download: bag.take("LastDownload"),
            completion_status: bag.take("CompletionStatus"),
            individual_address_loaded: bag.take("IndividualAddressLoaded"),
            application_program_loaded: bag.take("ApplicationProgramLoaded"),
            parameters_loaded: bag.take("ParametersLoaded"),
            communication_part_loaded: bag.take("CommunicationPartLoaded"),
            medium_config_loaded: bag.take("MediumConfigLoaded"),
            visibility_calculated: bag.take("IsActivityCalculated"),
            broken: bag.take("Broken"),
            parameters: Vec::new(),
            com_objects: Vec::new(),
            binary_data: Vec::new(),
            module_instances: Vec::new(),
            module_instances_raw: None,
            group_object_tree: Vec::new(),
            group_object_tree_raw: None,
            security_raw: None,
            other: Vec::new(),
        })),
        "ComObjectInstanceRef" => Frame::ComObject(SourceComObjectInstance {
            ref_id: bag.require("RefId", xpath)?,
            is_active: bag.take("IsActive"),
            datapoint_type: bag.take("DatapointType"),
            text: bag.take("Text"),
            description: bag.take("Description"),
            read_flag: bag.take("ReadFlag"),
            write_flag: bag.take("WriteFlag"),
            transmit_flag: bag.take("TransmitFlag"),
            update_flag: bag.take("UpdateFlag"),
            communication_flag: bag.take("CommunicationFlag"),
            sends: Vec::new(),
            receives: Vec::new(),
            links: bag
                .take("Links")
                .map(|s| s.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            channel_id: bag.take("ChannelId"),
            other: Vec::new(),
        }),
        "ParameterInstanceRef" => Frame::Parameter(SourceParameterInstance {
            ref_id: bag.require("RefId", xpath)?,
            value: bag.take("Value"),
        }),
        "BinaryData" => Frame::BinaryDataRef(SourceBinaryDataRef {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            other: Vec::new(),
        }),
        "BuildingPart" | "Space" => Frame::BuildingPart(SourceBuildingPart {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            number: bag.take("Number"),
            kind: bag.take("Type"),
            default_line: bag.take("DefaultLine"),
            completion_status: bag.take("CompletionStatus"),
            children: Vec::new(),
            device_refs: Vec::new(),
            other: Vec::new(),
        }),
        "GroupRange" => Frame::GroupRange(SourceGroupRange {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            range_start: bag.take("RangeStart"),
            range_end: bag.take("RangeEnd"),
            children: Vec::new(),
            addresses: Vec::new(),
            other: Vec::new(),
        }),
        "GroupAddress" => Frame::GroupAddress(SourceGroupAddress {
            id: bag.require("Id", xpath)?,
            name: bag.take("Name"),
            address: bag.take("Address"),
            central: bag.take("Central"),
            unfiltered: bag.take("Unfiltered"),
            other: Vec::new(),
        }),
        "ModuleInstance" => Frame::ModuleInstance(SourceModuleInstance {
            id: bag.require("Id", xpath)?,
            ref_id: bag.require("RefId", xpath)?,
            repeat_index: bag.take("RepeatIndex"),
            arguments: Vec::new(),
        }),
        "Argument" => Frame::Argument(SourceArgument {
            ref_id: bag.require("RefId", xpath)?,
            value: bag.take("Value"),
        }),
        _ => unreachable!("build_frame called for non-frame-bearing element {local}"),
    };
    Ok(attach_other(frame, unknown, bag.into_retained(xpath)))
}

/// Merges the unknown-attribute retentions and the known-but-unmapped
/// leftovers into the frame's `other` field.
fn attach_other(
    mut frame: Frame,
    unknown: Vec<RetainedAttribute>,
    leftover: Vec<RetainedAttribute>,
) -> Frame {
    let other = match &mut frame {
        Frame::Installation(v) => &mut v.other,
        Frame::Area(v) => &mut v.other,
        Frame::Line(v) => &mut v.other,
        Frame::Device(v) => &mut v.other,
        Frame::ComObject(v) => &mut v.other,
        Frame::BuildingPart(v) => &mut v.other,
        Frame::GroupRange(v) => &mut v.other,
        Frame::GroupAddress(v) => &mut v.other,
        // `BinaryData`'s leaf form does carry more than `Id`/`Name` at
        // schema ≥21 (`DoNotCopy`, measured on the ETS 6.3.0 reference
        // project), so it keeps what it is not asked about.
        Frame::BinaryDataRef(v) => &mut v.other,
        // `SourceParameterInstance`, `SourceArgument` and
        // `SourceModuleInstance` have no `other` field: every attribute any
        // of the three's known-element table entry lists is already modeled,
        // and each is a small enough leaf that adding a catch-all bucket for
        // a case that has never yet occurred is not worth it.
        Frame::Parameter(_) | Frame::Argument(_) | Frame::ModuleInstance(_) => return frame,
    };
    other.extend(unknown);
    other.extend(leftover);
    frame
}

/// Pops a just-closed frame's parent context and merges it into the right
/// field of whatever is now on top of the frame stack — or, for
/// `Installation`, into the document directly, since nothing above it in
/// the hierarchy ever pushes a frame.
fn attach_frame(
    frames: &mut [Frame],
    document: &mut SourceDocument,
    parent_name: &str,
    local: &str,
    closed: Frame,
) {
    match local {
        "Installation" => {
            document.installations.push(into_installation(closed));
        }
        "Area" => match frames.last_mut() {
            Some(Frame::Installation(i)) => i.areas.push(into_area(closed)),
            _ => unreachable!("Area outside an Installation frame"),
        },
        "Line" => match frames.last_mut() {
            Some(Frame::Area(a)) => a.lines.push(into_line(closed)),
            _ => unreachable!("Line outside an Area frame"),
        },
        "DeviceInstance" => match parent_name {
            "Line" => match frames.last_mut() {
                Some(Frame::Line(l)) => l.devices.push(into_device(closed)),
                _ => unreachable!("DeviceInstance outside a Line frame"),
            },
            "Segment" => match frames.last_mut() {
                Some(Frame::Line(l)) => l.devices.push(into_device(closed)),
                _ => unreachable!("DeviceInstance outside a Segment/Line frame"),
            },
            "UnassignedDevices" => match frames.last_mut() {
                Some(Frame::Installation(i)) => i.unassigned_devices.push(into_device(closed)),
                _ => unreachable!("DeviceInstance outside an Installation frame"),
            },
            other => unreachable!("DeviceInstance under unexpected parent {other}"),
        },
        "ComObjectInstanceRef" => match frames.last_mut() {
            Some(Frame::Device(d)) => d.com_objects.push(into_com_object(closed)),
            _ => unreachable!("ComObjectInstanceRef outside a DeviceInstance frame"),
        },
        "ParameterInstanceRef" => match frames.last_mut() {
            Some(Frame::Device(d)) => d.parameters.push(into_parameter(closed)),
            _ => unreachable!("ParameterInstanceRef outside a DeviceInstance frame"),
        },
        "BinaryData" => match frames.last_mut() {
            Some(Frame::Device(d)) => d.binary_data.push(into_binary_data(closed)),
            _ => unreachable!("BinaryData outside a DeviceInstance frame"),
        },
        "BuildingPart" | "Space" => match parent_name {
            "Buildings" | "Locations" => match frames.last_mut() {
                Some(Frame::Installation(i)) => i.buildings.push(into_building_part(closed)),
                _ => unreachable!("BuildingPart outside an Installation frame"),
            },
            "BuildingPart" | "Space" => match frames.last_mut() {
                Some(Frame::BuildingPart(b)) => b.children.push(into_building_part(closed)),
                _ => unreachable!("nested BuildingPart outside a BuildingPart frame"),
            },
            other => unreachable!("BuildingPart under unexpected parent {other}"),
        },
        "GroupRange" => match parent_name {
            "GroupRanges" => match frames.last_mut() {
                Some(Frame::Installation(i)) => i.group_ranges.push(into_group_range(closed)),
                _ => unreachable!("GroupRange outside an Installation frame"),
            },
            "GroupRange" => match frames.last_mut() {
                Some(Frame::GroupRange(g)) => g.children.push(into_group_range(closed)),
                _ => unreachable!("nested GroupRange outside a GroupRange frame"),
            },
            other => unreachable!("GroupRange under unexpected parent {other}"),
        },
        "GroupAddress" => match frames.last_mut() {
            Some(Frame::GroupRange(g)) => g.addresses.push(into_group_address(closed)),
            _ => unreachable!("GroupAddress outside a GroupRange frame"),
        },
        "ModuleInstance" => match frames.last_mut() {
            Some(Frame::Device(d)) => d.module_instances.push(into_module_instance(closed)),
            _ => unreachable!("ModuleInstance outside a DeviceInstance frame"),
        },
        "Argument" => match frames.last_mut() {
            Some(Frame::ModuleInstance(m)) => m.arguments.push(into_argument(closed)),
            _ => unreachable!("Argument outside a ModuleInstance frame"),
        },
        other => unreachable!("attach_frame called for non-frame-bearing element {other}"),
    }
}

fn into_installation(f: Frame) -> SourceInstallation {
    match f {
        Frame::Installation(v) => v,
        _ => unreachable!(),
    }
}
fn into_area(f: Frame) -> SourceArea {
    match f {
        Frame::Area(v) => v,
        _ => unreachable!(),
    }
}
fn into_line(f: Frame) -> SourceLine {
    match f {
        Frame::Line(v) => v,
        _ => unreachable!(),
    }
}
fn into_device(f: Frame) -> SourceDevice {
    match f {
        Frame::Device(v) => *v,
        _ => unreachable!(),
    }
}
fn into_com_object(f: Frame) -> SourceComObjectInstance {
    match f {
        Frame::ComObject(v) => v,
        _ => unreachable!(),
    }
}
fn into_parameter(f: Frame) -> SourceParameterInstance {
    match f {
        Frame::Parameter(v) => v,
        _ => unreachable!(),
    }
}
fn into_binary_data(f: Frame) -> SourceBinaryDataRef {
    match f {
        Frame::BinaryDataRef(v) => v,
        _ => unreachable!(),
    }
}
fn into_building_part(f: Frame) -> SourceBuildingPart {
    match f {
        Frame::BuildingPart(v) => v,
        _ => unreachable!(),
    }
}
fn into_group_range(f: Frame) -> SourceGroupRange {
    match f {
        Frame::GroupRange(v) => v,
        _ => unreachable!(),
    }
}
fn into_group_address(f: Frame) -> SourceGroupAddress {
    match f {
        Frame::GroupAddress(v) => v,
        _ => unreachable!(),
    }
}
fn into_module_instance(f: Frame) -> SourceModuleInstance {
    match f {
        Frame::ModuleInstance(v) => v,
        _ => unreachable!(),
    }
}
fn into_argument(f: Frame) -> SourceArgument {
    match f {
        Frame::Argument(v) => v,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::known::known_schema;
    use crate::testutil::reference_kv_schema21_path;
    use crate::Container;

    #[test]
    fn the_kv_sample_parses_with_a_bounded_unknown_count() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        // Not yet zero — Task 3's table is filled in iteratively against this
        // exact test's failure output. This test's job in this task is to
        // prove the state machine itself does not panic or error on the real
        // file; Task 8 tightens the assertion to `== vec![]`.
        let mut c = Container::open(std::fs::read(reference_kv_schema21_path()).unwrap()).unwrap();
        let bytes = c.read("P-03DE/0.xml").unwrap();
        let out =
            parse_installation_v21(&bytes, "P-03DE/0.xml", known_schema(21).unwrap()).unwrap();
        assert_eq!(
            out.document.installations[0]
                .areas
                .iter()
                .flat_map(|a| &a.lines)
                .map(|l| l.devices.len())
                .sum::<usize>(),
            4
        );
    }

    #[test]
    fn a_module_based_device_carries_its_module_instances_and_group_object_tree() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(std::fs::read(reference_kv_schema21_path()).unwrap()).unwrap();
        let bytes = c.read("P-03DE/0.xml").unwrap();
        let out =
            parse_installation_v21(&bytes, "P-03DE/0.xml", known_schema(21).unwrap()).unwrap();
        let device = out.document.installations[0]
            .areas
            .iter()
            .flat_map(|a| &a.lines)
            .flat_map(|l| &l.devices)
            .find(|d| !d.module_instances.is_empty())
            .unwrap();
        assert_eq!(
            device.module_instances[0].repeat_index.as_deref(),
            Some("6x1")
        );
        assert!(!device.group_object_tree.is_empty());
        assert!(device.module_instances_raw.is_some());
        assert!(device.group_object_tree_raw.is_some());
        // Regression test for a review finding: this attribute is named
        // `IsActivityCalculated` at schema >=21, not schema 11's
        // `IsCommunicationObjectVisibilityCalculated` — every device in this
        // sample carries `IsActivityCalculated="true"`.
        assert_eq!(device.visibility_calculated.as_deref(), Some("true"));
    }

    /// Regression test for a review finding on this task: `Security` was
    /// initially retained in the document-wide `retained_elements` bucket,
    /// indistinguishable between devices (every device's `Security` shares
    /// the same structural xpath). It must be addressable per device instead
    /// — Task 7's export brief assumes exactly that.
    #[test]
    fn every_device_carries_its_own_security_element_raw() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(std::fs::read(reference_kv_schema21_path()).unwrap()).unwrap();
        let bytes = c.read("P-03DE/0.xml").unwrap();
        let out =
            parse_installation_v21(&bytes, "P-03DE/0.xml", known_schema(21).unwrap()).unwrap();
        let devices: Vec<_> = out.document.installations[0]
            .areas
            .iter()
            .flat_map(|a| &a.lines)
            .flat_map(|l| &l.devices)
            .collect();
        assert_eq!(devices.len(), 4);
        for device in &devices {
            let security = device
                .security_raw
                .as_ref()
                .unwrap_or_else(|| panic!("device {} has no security_raw", device.id));
            assert!(security.raw.starts_with(b"<Security"));
            assert!(security.raw.ends_with(b"/>") || security.raw.ends_with(b"</Security>"));
        }
        // Never the document-wide unknown/opaque bucket — it belongs to its
        // own device now, not the flat catch-all.
        assert!(out.retained_elements.iter().all(|e| e.name != "Security"));
    }
}
