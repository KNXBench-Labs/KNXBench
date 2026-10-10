//! The element dispatch for `0.xml`: one small state machine over SAX
//! events, building the nested `SourceDocument` shape as elements open and
//! close.
//!
//! Two stacks are threaded through the loop: `path_stack`, the absolute
//! element path (every element that goes through the normal open/close
//! cycle pushes its local name and pops it again), and `frames`, the
//! in-progress struct values waiting for their children to finish — pushed
//! only by elements the known-element table models as a struct. Wrapper
//! elements (`Installations`, `Topology`, `ParameterInstanceRefs`, …) push
//! `path_stack` but never `frames`; when a frame-bearing element's sibling
//! closes, the frame stack's top is therefore always its true structural
//! parent, wrapper elements notwithstanding.
//!
//! `GroupRange` and `BuildingPart` nest inside themselves. The known-element
//! table registers one path per element, so a nested occurrence's absolute
//! path (e.g. `.../GroupRange/GroupRange`) is collapsed to the same path as
//! the outer one before it is looked up in the table; the *uncollapsed*
//! path is still what gets reported in `UnknownConstruct`/`RetainedAttribute`
//! xpaths, since that is more useful to a human reading the import report.

use super::observed_reader::ObservedReader as Reader;
use quick_xml::events::{BytesStart, Event};

use crate::known::KnownSchema;
use crate::source::{
    RetainedAttribute, RetainedElement, SourceArea, SourceBinaryDataRef, SourceBuildingPart,
    SourceComObjectInstance, SourceDevice, SourceDocument, SourceGroupAddress, SourceGroupRange,
    SourceInstallation, SourceLine, SourceParameterInstance,
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
}

/// Whether an element that went through the normal open/close cycle pushed
/// a [`Frame`] or was a pure wrapper. Recorded at open time and consulted at
/// close time so the ambiguous case — `BinaryData`, which is a wrapper as
/// `DeviceInstance`'s child and a leaf as its own child — never needs a
/// special case: the close side just does what the open side decided.
enum Kind {
    Wrapper,
    Frame,
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

/// The only two elements that genuinely nest inside themselves at any
/// depth, sharing one known-element table entry regardless of how deep.
/// `BinaryData` also repeats its own local name at consecutive stack
/// positions (the wrapper, then the leaf), but those are two *different*
/// table entries with different attributes — collapsing them would look up
/// the wrapper's (empty) attribute list for the leaf. Only these two names
/// collapse.
const SELF_RECURSIVE: &[&str] = &["GroupRange", "BuildingPart"];

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

/// Parses one `0.xml` document (an installation's topology, devices and
/// group addresses) into a [`SourceDocument`], tolerantly: an element or
/// attribute the known-element table for `schema` does not list is retained
/// and reported, never a fatal error on its own. Only malformed XML and a
/// missing required identity attribute (`Id`/`RefId` on a modeled element)
/// fail the whole parse.
pub fn parse_installation(
    bytes: &[u8],
    source_path: &str,
    schema: &KnownSchema,
) -> Result<ParseOutput, ParseError> {
    // `trim_text` stays off (the default): a whitespace-only run between
    // tags must still surface as its own `Event::Text`, so `pos_before`,
    // snapshotted once per loop iteration, always lands right before the
    // next real tag's `<` rather than before invisibly-skipped whitespace.
    // That exactness matters here — it is what lets an unknown element's
    // byte span be captured verbatim, tag and all.
    //
    // The reference project's own `0.xml` opens with a UTF-8 BOM.
    // `Reader::from_reader` strips it internally (`remove_utf8_bom`) by
    // sliding its *own* view of the input forward — `reader.buffer_position()`
    // then counts from 0 at the first byte *after* the BOM, not from 0 at
    // the first byte of `bytes` itself. Indexing `bytes[pos_before..end]`
    // with those reader-reported positions against the untouched `bytes`
    // slice silently reads a window shifted 3 bytes early: it swallows 3
    // bytes of whatever precedes the real span and drops the span's own
    // last 3 bytes. Found via Task 18's export round-trip, where a
    // retained `BusAccess` element came back missing its closing ` />` —
    // Task 6's own tests never caught it, since none inspects a captured
    // span's content against the real (BOM-carrying) reference file, only
    // the hand-written `MINIMAL` fixture (no BOM) and unknown-count
    // assertions. Stripping the BOM here, once, before the reader and every
    // `bytes[..]` index share the same baseline, fixes the offset at its
    // source rather than patching each call site.
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
                if let Kind::Frame = kind {
                    let parent_name = path_stack.last().map(String::as_str).unwrap_or("");
                    let closed = frames
                        .pop()
                        .expect("frame stack out of sync with path stack");
                    attach_frame(&mut frames, &mut document, parent_name, &local, closed);
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

    document.xml_observations = reader.counts;
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
/// string references, the verbatim-retained `BusAccess`, and anything the
/// table does not know at all).
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
        | "GroupAddresses"
        | "GroupRanges" => true,
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
        ("BuildingPart", "DeviceInstanceRef") => match frames.last_mut() {
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
            visibility_calculated: bag.take("IsCommunicationObjectVisibilityCalculated"),
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
            links: Vec::new(),
            channel_id: None,
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
        "BuildingPart" => Frame::BuildingPart(SourceBuildingPart {
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
            datapoint_type: None,
            other: Vec::new(),
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
        Frame::BinaryDataRef(v) => &mut v.other,
        // `SourceParameterInstance` has no `other` field: it is a two-field
        // leaf and both attributes its table lists are already modeled.
        Frame::Parameter(_) => return frame,
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
        "BuildingPart" => match parent_name {
            "Buildings" => match frames.last_mut() {
                Some(Frame::Installation(i)) => i.buildings.push(into_building_part(closed)),
                _ => unreachable!("BuildingPart outside an Installation frame"),
            },
            "BuildingPart" => match frames.last_mut() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::known::known_schema;
    use crate::Container;

    fn reference_ets4_bytes() -> Vec<u8> {
        std::fs::read(knx_testsupport::reference_ets4_path())
            .expect("reference ETS4 project lives under OriginalData/ (gitignored); see knx_testsupport::corpus_available")
    }

    const MINIMAL: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" DefaultLine="P-0001-0_L-2" CompletionStatus="Undefined">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="A" Address="1" CompletionStatus="Undefined">
            <Line Id="P-0001-0_L-2" Name="L" Address="1" MediumTypeRefId="MT-0" CompletionStatus="Accepted">
              <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                              LastModified="2023-07-14T11:55:33" CompletionStatus="FinishedDesign"
                              IndividualAddressLoaded="1" ApplicationProgramLoaded="1"
                              ParametersLoaded="1" CommunicationPartLoaded="1"
                              MediumConfigLoaded="1" IsCommunicationObjectVisibilityCalculated="1"
                              Broken="0">
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="M-0001_A-1_O-0_R-1" DatapointType="" IsActive="1">
                    <Connectors><Send GroupAddressRefId="P-0001-0_GA-1" /></Connectors>
                  </ComObjectInstanceRef>
                </ComObjectInstanceRefs>
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupRange Id="P-0001-0_GR-2" Name="An/Aus" RangeStart="1" RangeEnd="127">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

    #[test]
    fn a_minimal_document_parses_into_the_source_shape() {
        let out = parse_installation(MINIMAL, "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
        let inst = &out.document.installations[0];
        assert_eq!(out.document.project_id, "P-0001");
        assert_eq!(inst.areas[0].lines[0].devices.len(), 1);
        let com = &inst.areas[0].lines[0].devices[0].com_objects[0];
        assert_eq!(com.sends, vec!["P-0001-0_GA-1"]);
        assert!(com.receives.is_empty());
        assert_eq!(
            inst.group_ranges[0].children[0].addresses[0].id,
            "P-0001-0_GA-1"
        );
        assert!(out.unknown.is_empty());
    }

    #[test]
    fn an_empty_attribute_value_is_kept_as_an_empty_string_not_dropped() {
        let out = parse_installation(MINIMAL, "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
        let com = &out.document.installations[0].areas[0].lines[0].devices[0].com_objects[0];
        assert_eq!(com.datapoint_type.as_deref(), Some(""));
        assert_eq!(com.text, None);
    }

    #[test]
    fn an_unknown_attribute_is_reported_with_its_value_and_not_fatal() {
        let xml = String::from_utf8(MINIMAL.to_vec()).unwrap().replace(
            r#"MediumTypeRefId="MT-0" CompletionStatus="Accepted""#,
            r#"MediumTypeRefId="MT-0" CompletionStatus="Accepted" Puid="42""#,
        );
        let out =
            parse_installation(xml.as_bytes(), "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
        let u = out.unknown.iter().find(|u| u.name == "Puid").unwrap();
        assert_eq!(u.kind, UnknownKind::Attribute);
        assert_eq!(
            u.xpath,
            "/KNX/Project/Installations/Installation/Topology/Area/Line"
        );
        assert_eq!(u.sample.as_deref(), Some("42"));
        let line = &out.document.installations[0].areas[0].lines[0];
        assert!(line
            .other
            .iter()
            .any(|a| a.name == "Puid" && a.value == "42"));
    }

    #[test]
    fn an_unknown_element_is_retained_verbatim_and_reported() {
        let xml = String::from_utf8(MINIMAL.to_vec())
            .unwrap()
            .replace("<Topology>", "<Security SequenceNumber=\"7\"/><Topology>");
        let out =
            parse_installation(xml.as_bytes(), "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
        let u = out.unknown.iter().find(|u| u.name == "Security").unwrap();
        assert_eq!(u.kind, UnknownKind::Element);
        let kept = out
            .retained_elements
            .iter()
            .find(|e| e.name == "Security")
            .unwrap();
        assert_eq!(kept.raw, br#"<Security SequenceNumber="7"/>"#);
    }

    #[test]
    fn repeated_unknown_attributes_aggregate_into_one_report_line() {
        let xml = String::from_utf8(MINIMAL.to_vec()).unwrap().replace(
            r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />"#,
            r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" Puid="1" />
           <GroupAddress Id="P-0001-0_GA-2" Address="2" Name="GB" Puid="2" />"#,
        );
        let out =
            parse_installation(xml.as_bytes(), "P-0001/0.xml", known_schema(11).unwrap()).unwrap();
        let u = out.unknown.iter().find(|u| u.name == "Puid").unwrap();
        assert_eq!(u.occurrences, 2);
    }

    #[test]
    fn truncated_xml_is_a_parse_error_carrying_its_byte_position() {
        let truncated = &MINIMAL[..MINIMAL.len() / 2];
        assert!(matches!(
            parse_installation(truncated, "P-0001/0.xml", known_schema(11).unwrap()),
            Err(ParseError::Xml { .. })
        ));
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn the_reference_project_parses_with_no_unknown_constructs() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let bytes = c.read("P-0512/0.xml").unwrap();
        let out = parse_installation(&bytes, "P-0512/0.xml", known_schema(11).unwrap()).unwrap();
        assert_eq!(
            out.unknown,
            vec![],
            "the schema-11 table was transcribed from this very project; \
             anything unknown here is a gap in the table"
        );
        let inst = &out.document.installations[0];
        let devices: usize = inst
            .areas
            .iter()
            .flat_map(|a| &a.lines)
            .map(|l| l.devices.len())
            .sum();
        assert_eq!(devices + inst.unassigned_devices.len(), 36);
    }
}
