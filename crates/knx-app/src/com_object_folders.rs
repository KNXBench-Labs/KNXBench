//! Places communication objects in the saved `GroupObjectTree` folders they were imported with.
//!
//! `[D]` Project Schema 23 v01.00.00 §1.2.5.21–23 (printed pages 51–52):
//! `DeviceInstance/GroupObjectTree` holds `Node` elements (nested through
//! `Nodes`); `Node/@Type` is `Folder` (a `ParameterBlock` with
//! `ShowInComObjectTree`) or `Channel`, `Node/@RefId` is required and names
//! that channel or block by its shortened id, and `@GroupObjectInstances`
//! lists the objects of that node. The tree's own `@GroupObjectInstances`
//! lists the objects of the `ChannelIndependentBlock`.
//!
//! The importer keeps every device's tree verbatim as an opaque
//! `RetainedElement`; this module only reads that retained copy. It never
//! resolves a `RefId` against product data, so a folder is shown by its
//! declared id chain, not by a manufacturer label (WP-06: manufacturer
//! `ShowInComObjectTree` mapping is not admitted). Anything the declaration
//! does not settle is refused, never guessed: a device whose tree cannot be
//! attributed or read, an object listed more than once, a node without a
//! unique declared identity, a module object and a stored-inactive object get
//! no folder, each with its own reason. Nothing here writes to the project.

use std::collections::{BTreeMap, HashMap};

use knx_core::{ComObjectInstanceId, DeviceId, Project, SourceRef};
use knx_store::StoredOpaqueEntry;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};

/// The opaque row kind and name the importer gives a retained tree.
const RETAINED_ELEMENT_KIND: &str = "RetainedElement";
const TREE_NAME: &str = "GroupObjectTree";
/// Bounds for reading one retained tree; beyond them the tree is refused.
const MAX_TREE_BYTES: usize = 16 * 1024 * 1024;
const MAX_TREE_DEPTH: usize = 64;
const MAX_TREE_EVENTS: usize = 2_000_000;

/// `Node/@Type`, as Project Schema 23 §1.2.5.23 lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeNodeKind {
    Channel,
    Folder,
}

/// One declared node on the way from the tree root to an object.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeNodeRef {
    pub kind: TreeNodeKind,
    /// `Node/@RefId`, verbatim (a shortened channel or parameter-block id).
    pub ref_id: String,
}

/// Where the saved tree places one object, or why it shows no folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedFolder {
    /// Listed exactly once, in a `Folder` node whose whole ancestry declares
    /// a unique identity. `path` runs from the root to that folder.
    Folder { path: Vec<TreeNodeRef> },
    /// Listed exactly once, at the root or directly in a `Channel` node.
    NoFolder,
    /// Not listed: the device has no retained tree (not imported, schema 11,
    /// created in KNXBench), the object has no source identity, or the object
    /// is not in the tree.
    NotInSavedTree,
    /// Listed more than once in the device's tree.
    Ambiguous,
    /// Listed in, or below, a node without a `Type` of `Channel`/`Folder`,
    /// without a `RefId`, or with the same declared path as another node.
    UnidentifiedNode,
    /// The tree or object cannot be attributed exactly (several devices or
    /// objects share a source identity, the object names another owner/source
    /// member, or several trees match), or the retained hash/bytes cannot be
    /// verified and read.
    TreeUnresolved,
    /// A module-instance object that the tree places in a folder: binding a
    /// folder through a module instance is not admitted (WP-07).
    ModuleObject,
    /// An object the project stores as inactive that the tree places in a
    /// folder: the saved tree describes the active objects only.
    Inactive,
}

#[derive(Debug, Clone)]
struct ObjectSource {
    source: SourceRef,
    owner: DeviceId,
    unique_source: bool,
    module: bool,
    stored_active: bool,
}

/// What placing one device's objects needs from the project.
#[derive(Debug, Clone)]
pub struct FolderInput {
    device: DeviceId,
    source: SourceRef,
    /// Another device states the same source identity.
    shared_source: bool,
    objects: Vec<(ComObjectInstanceId, ObjectSource)>,
}

impl FolderInput {
    /// `None` when the device does not exist.
    pub fn collect(project: &Project, device: DeviceId) -> Option<Self> {
        let dev = project.devices.get(device)?;
        let mut object_source_counts = HashMap::new();
        for object in project.devices.com_objects() {
            *object_source_counts
                .entry((object.source.path.as_str(), object.source.ets_id.as_str()))
                .or_insert(0usize) += 1;
        }
        let shared_source = project
            .devices
            .iter()
            .any(|other| other.id != dev.id && other.source == dev.source);
        let objects = dev
            .com_objects
            .iter()
            .filter_map(|id| project.devices.com_object(*id))
            .map(|com| {
                (
                    com.id,
                    ObjectSource {
                        source: com.source.clone(),
                        owner: com.device,
                        unique_source: object_source_counts
                            .get(&(com.source.path.as_str(), com.source.ets_id.as_str()))
                            == Some(&1),
                        module: com.module_instance.is_some()
                            || com.source.ets_id.starts_with("MD-"),
                        stored_active: com.is_active,
                    },
                )
            })
            .collect();
        Some(Self {
            device,
            source: dev.source.clone(),
            shared_source,
            objects,
        })
    }
}

/// Every object of the device in `input`, placed by its retained tree.
pub fn saved_folders(
    input: &FolderInput,
    opaque: &[StoredOpaqueEntry],
) -> BTreeMap<ComObjectInstanceId, SavedFolder> {
    let everyone = |folder: SavedFolder| {
        input
            .objects
            .iter()
            .map(|(id, _)| (*id, folder.clone()))
            .collect()
    };
    if input.source.path.is_empty() || input.source.ets_id.is_empty() {
        return everyone(SavedFolder::NotInSavedTree);
    }
    let suffix = format!("/DeviceInstance[@Id='{}']/{TREE_NAME}", input.source.ets_id);
    let mut trees = opaque.iter().filter(|entry| {
        entry.kind == RETAINED_ELEMENT_KIND
            && entry.name == TREE_NAME
            && entry.source_path == input.source.path
            && entry.xpath.ends_with(&suffix)
    });
    let Some(tree) = trees.next() else {
        return everyone(SavedFolder::NotInSavedTree);
    };
    if trees.next().is_some() || input.shared_source {
        return everyone(SavedFolder::TreeUnresolved);
    }
    if knx_store::manufacturer_file_hash(&tree.bytes) != tree.sha256 {
        return everyone(SavedFolder::TreeUnresolved);
    }
    let Ok(listed) = read_tree(&tree.bytes) else {
        return everyone(SavedFolder::TreeUnresolved);
    };
    input
        .objects
        .iter()
        .map(|(id, object)| {
            let placement = if object.source.path.is_empty() || object.source.ets_id.is_empty() {
                SavedFolder::NotInSavedTree
            } else if object.owner != input.device
                || object.source.path != input.source.path
                || !object.unique_source
            {
                SavedFolder::TreeUnresolved
            } else {
                place(object, listed.get(object.source.ets_id.as_str()))
            };
            (*id, placement)
        })
        .collect()
}

fn place(object: &ObjectSource, listed: Option<&Vec<Placement>>) -> SavedFolder {
    let placement = match listed.map(Vec::as_slice) {
        None | Some([]) => return SavedFolder::NotInSavedTree,
        Some([single]) => single,
        Some(_) => return SavedFolder::Ambiguous,
    };
    let Placement::Declared(path) = placement else {
        return SavedFolder::UnidentifiedNode;
    };
    match path.last() {
        Some(node) if node.kind == TreeNodeKind::Folder => {
            if object.module {
                SavedFolder::ModuleObject
            } else if !object.stored_active {
                SavedFolder::Inactive
            } else {
                SavedFolder::Folder { path: path.clone() }
            }
        }
        _ => SavedFolder::NoFolder,
    }
}

/// One listing of an object: the declared node path, or no usable identity.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Placement {
    Declared(Vec<TreeNodeRef>),
    Unidentified,
}

#[derive(Debug)]
struct TreeUnreadable;

/// One open element while reading: the root, a `Nodes` wrapper, or a `Node`
/// with its declared path (`None` once it or an ancestor has no identity).
enum Frame {
    Root,
    Nodes(Option<Vec<TreeNodeRef>>),
    Node(Option<Vec<TreeNodeRef>>),
}

/// Reads a retained tree into object id -> every listing of that id.
///
/// Strict on structure: the root must be `GroupObjectTree` and every element
/// below it a `Nodes` or `Node` in the project namespace, in the positions
/// §1.2.5.21–23 gives them. An unprefixed element without a namespace
/// declaration inside a wholly unbound fragment inherits the namespace the
/// importer accepted the tree in. An explicitly namespaced fragment must use
/// the same namespace on every structural element. Anything else (a mixed or
/// foreign namespace, an unbound prefix, a DTD, malformed XML) refuses the
/// whole tree rather than reading around it.
fn read_tree(bytes: &[u8]) -> Result<HashMap<String, Vec<Placement>>, TreeUnreadable> {
    if bytes.len() > MAX_TREE_BYTES {
        return Err(TreeUnreadable);
    }
    let mut reader = NsReader::from_reader(bytes);
    let mut stack: Vec<Frame> = Vec::new();
    let mut listed: HashMap<String, Vec<Placement>> = HashMap::new();
    let mut node_paths: BTreeMap<Vec<TreeNodeRef>, usize> = BTreeMap::new();
    let mut tree_namespace = None;
    let mut root_seen = false;
    let mut events = 0usize;
    loop {
        events += 1;
        if events > MAX_TREE_EVENTS || stack.len() > MAX_TREE_DEPTH {
            return Err(TreeUnreadable);
        }
        let (ns, event) = reader.read_resolved_event().map_err(|_| TreeUnreadable)?;
        let (element, empty) = match &event {
            Event::Start(element) => (element, false),
            Event::Empty(element) => (element, true),
            Event::End(_) => {
                stack.pop().ok_or(TreeUnreadable)?;
                continue;
            }
            Event::DocType(_) => return Err(TreeUnreadable),
            Event::Eof => break,
            _ => continue,
        };
        let namespace = project_namespace(&ns).ok_or(TreeUnreadable)?;
        if tree_namespace
            .as_ref()
            .is_some_and(|expected| expected != &namespace)
        {
            return Err(TreeUnreadable);
        }
        tree_namespace.get_or_insert(namespace);
        let local = element.local_name();
        let frame = match (stack.last(), local.as_ref()) {
            (None, "GroupObjectTree") if !root_seen => {
                root_seen = true;
                record(&mut listed, element, &Placement::Declared(Vec::new()))?;
                Frame::Root
            }
            (Some(Frame::Root), "Nodes") => Frame::Nodes(Some(Vec::new())),
            (Some(Frame::Node(path)), "Nodes") => Frame::Nodes(path.clone()),
            (Some(Frame::Root), "Node") => node(&mut listed, &mut node_paths, element, Some(&[]))?,
            (Some(Frame::Nodes(path)), "Node") => {
                node(&mut listed, &mut node_paths, element, path.as_deref())?
            }
            _ => return Err(TreeUnreadable),
        };
        if !empty {
            stack.push(frame);
        }
    }
    if !root_seen || !stack.is_empty() {
        return Err(TreeUnreadable);
    }
    // A path declared by two nodes identifies neither of them, nor anything
    // below them.
    let repeated: Vec<&Vec<TreeNodeRef>> = node_paths
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(path, _)| path)
        .collect();
    for placements in listed.values_mut() {
        for placement in placements.iter_mut() {
            if let Placement::Declared(path) = placement {
                if repeated.iter().any(|prefix| path.starts_with(prefix)) {
                    *placement = Placement::Unidentified;
                }
            }
        }
    }
    Ok(listed)
}

/// The retained fragment either omits one inherited project namespace on
/// every element, or explicitly resolves every element to one project
/// namespace URI. The two modes must never be mixed.
#[derive(Debug, PartialEq, Eq)]
enum ProjectNamespace {
    InheritedOmitted,
    Explicit(String),
}

fn project_namespace(ns: &ResolveResult) -> Option<ProjectNamespace> {
    match ns {
        ResolveResult::Unbound => Some(ProjectNamespace::InheritedOmitted),
        ResolveResult::Bound(bound) => {
            let namespace = bound.as_ref();
            namespace
                .strip_prefix("http://knx.org/xml/project/")
                .filter(|version| {
                    !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit())
                })
                .map(|_| ProjectNamespace::Explicit(namespace.to_owned()))
        }
        ResolveResult::Unknown(_) => None,
    }
}

fn node(
    listed: &mut HashMap<String, Vec<Placement>>,
    node_paths: &mut BTreeMap<Vec<TreeNodeRef>, usize>,
    element: &BytesStart,
    parent: Option<&[TreeNodeRef]>,
) -> Result<Frame, TreeUnreadable> {
    let kind = match attribute(element, "Type")?.as_deref() {
        Some("Channel") => Some(TreeNodeKind::Channel),
        Some("Folder") => Some(TreeNodeKind::Folder),
        _ => None,
    };
    let ref_id = attribute(element, "RefId")?.filter(|value| !value.is_empty());
    let path = match (parent, kind, ref_id) {
        (Some(parent), Some(kind), Some(ref_id)) => {
            let mut path = parent.to_vec();
            path.push(TreeNodeRef { kind, ref_id });
            *node_paths.entry(path.clone()).or_default() += 1;
            Some(path)
        }
        _ => None,
    };
    let placement = match &path {
        Some(path) => Placement::Declared(path.clone()),
        None => Placement::Unidentified,
    };
    record(listed, element, &placement)?;
    Ok(Frame::Node(path))
}

/// Adds every id of the element's own `GroupObjectInstances` at `placement`.
fn record(
    listed: &mut HashMap<String, Vec<Placement>>,
    element: &BytesStart,
    placement: &Placement,
) -> Result<(), TreeUnreadable> {
    if let Some(ids) = attribute(element, "GroupObjectInstances")? {
        for id in ids.split_ascii_whitespace() {
            listed
                .entry(id.to_owned())
                .or_default()
                .push(placement.clone());
        }
    }
    Ok(())
}

/// An unprefixed attribute's normalized value; prefixed attributes are not
/// the schema's and are not read.
fn attribute(element: &BytesStart, name: &str) -> Result<Option<String>, TreeUnreadable> {
    let mut found = None;
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_| TreeUnreadable)?;
        if attribute.key.prefix().is_some() || attribute.key.local_name().as_ref() != name {
            continue;
        }
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| TreeUnreadable)?
            .into_owned();
        if found.replace(value).is_some() {
            return Err(TreeUnreadable);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        ComObjectInstance, CommissioningState, DeviceInstance, Language, Override, ResolvedFlags,
    };

    fn folder(path: &[(TreeNodeKind, &str)]) -> SavedFolder {
        SavedFolder::Folder {
            path: path
                .iter()
                .map(|(kind, ref_id)| TreeNodeRef {
                    kind: *kind,
                    ref_id: (*ref_id).to_owned(),
                })
                .collect(),
        }
    }

    fn input(objects: &[(&str, bool, bool)]) -> FolderInput {
        FolderInput {
            device: DeviceId(1),
            source: SourceRef {
                path: "P-0001/0.xml".into(),
                ets_id: "P-0001-0_DI-1".into(),
            },
            shared_source: false,
            objects: objects
                .iter()
                .enumerate()
                .map(|(index, (ets_id, module, stored_active))| {
                    (
                        ComObjectInstanceId(index as u32 + 1),
                        ObjectSource {
                            source: SourceRef {
                                path: "P-0001/0.xml".into(),
                                ets_id: (*ets_id).to_owned(),
                            },
                            owner: DeviceId(1),
                            unique_source: true,
                            module: *module,
                            stored_active: *stored_active,
                        },
                    )
                })
                .collect(),
        }
    }

    fn tree_row(source_path: &str, device: &str, bytes: &str) -> StoredOpaqueEntry {
        StoredOpaqueEntry {
            source_path: source_path.into(),
            xpath: format!(
                "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance[@Id='{device}']/GroupObjectTree"
            ),
            kind: RETAINED_ELEMENT_KIND.into(),
            name: TREE_NAME.into(),
            bytes: bytes.as_bytes().to_vec(),
            sha256: knx_store::manufacturer_file_hash(bytes.as_bytes()),
        }
    }

    fn placed(objects: &[(&str, bool, bool)], tree: &str) -> Vec<SavedFolder> {
        let input = input(objects);
        let rows = [tree_row("P-0001/0.xml", "P-0001-0_DI-1", tree)];
        saved_folders(&input, &rows).into_values().collect()
    }

    const DECLARED: &str = r#"<GroupObjectTree GroupObjectInstances="O-1_R-1"><Nodes><Node Type="Channel" RefId="CH-1" GroupObjectInstances="O-2_R-1"><Nodes><Node Type="Folder" RefId="PB-3" GroupObjectInstances="O-3_R-1 O-4_R-1"><Nodes><Node Type="Folder" RefId="PB-4" GroupObjectInstances="O-5_R-1"/></Nodes></Node></Nodes></Node><Node Type="Folder" RefId="PB-9" GroupObjectInstances="O-6_R-1"/></Nodes></GroupObjectTree>"#;

    fn project_with_object(source: SourceRef, owner: DeviceId) -> Project {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "P-0001/0.xml".into(),
                ets_id: "P-0001-0_DI-1".into(),
            },
            name: "Device".into(),
            description: None,
            address: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source,
            device: owner,
            number: 1,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        project
    }

    fn project_placement(project: &Project) -> SavedFolder {
        let input = FolderInput::collect(project, DeviceId(1)).unwrap();
        let rows = [tree_row(
            "P-0001/0.xml",
            "P-0001-0_DI-1",
            r#"<GroupObjectTree><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#,
        )];
        saved_folders(&input, &rows)[&ComObjectInstanceId(1)].clone()
    }

    #[test]
    fn object_source_must_uniquely_belong_to_the_device_and_source_member() {
        let source = |path: &str| SourceRef {
            path: path.into(),
            ets_id: "O-1_R-1".into(),
        };
        let expected_folder = folder(&[(TreeNodeKind::Folder, "PB-1")]);

        let valid = project_with_object(source("P-0001/0.xml"), DeviceId(1));
        let foreign_owner = project_with_object(source("P-0001/0.xml"), DeviceId(2));
        let other_member = project_with_object(source("P-0002/0.xml"), DeviceId(1));
        let missing_member = project_with_object(source(""), DeviceId(1));

        let mut duplicate = project_with_object(source("P-0001/0.xml"), DeviceId(1));
        let mut duplicate_object = duplicate
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .clone();
        duplicate_object.id = ComObjectInstanceId(2);
        duplicate.devices.insert_com_object(duplicate_object);

        let mut same_id_other_member = project_with_object(source("P-0001/0.xml"), DeviceId(1));
        let mut other_object = same_id_other_member
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .clone();
        other_object.id = ComObjectInstanceId(2);
        other_object.source.path = "P-0002/0.xml".into();
        same_id_other_member.devices.insert_com_object(other_object);

        assert_eq!(
            [
                project_placement(&valid),
                project_placement(&foreign_owner),
                project_placement(&other_member),
                project_placement(&missing_member),
                project_placement(&duplicate),
                project_placement(&same_id_other_member),
            ],
            [
                expected_folder.clone(),
                SavedFolder::TreeUnresolved,
                SavedFolder::TreeUnresolved,
                SavedFolder::NotInSavedTree,
                SavedFolder::TreeUnresolved,
                expected_folder,
            ]
        );
    }

    #[test]
    fn retained_tree_bytes_must_match_their_stored_hash() {
        let original = r#"<GroupObjectTree><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#;
        let mut row = tree_row("P-0001/0.xml", "P-0001-0_DI-1", original);
        row.bytes = r#"<GroupObjectTree><Node Type="Folder" RefId="PB-2" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#
            .as_bytes()
            .to_vec();

        assert_eq!(
            saved_folders(&input(&[("O-1_R-1", false, true)]), &[row])
                .into_values()
                .collect::<Vec<_>>(),
            [SavedFolder::TreeUnresolved]
        );
    }

    #[test]
    fn explicit_project_namespace_rejects_unbound_structural_children() {
        let objects = [("O-1_R-1", false, true)];
        let mixed = r#"<k:GroupObjectTree xmlns:k="http://knx.org/xml/project/23"><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></k:GroupObjectTree>"#;
        assert_eq!(placed(&objects, mixed), [SavedFolder::TreeUnresolved]);

        let inherited_fragment = r#"<GroupObjectTree><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#;
        assert_eq!(
            placed(&objects, inherited_fragment),
            [folder(&[(TreeNodeKind::Folder, "PB-1")])]
        );
    }

    #[test]
    fn declared_folders_place_objects_by_their_ref_id_chain() {
        let objects = [
            ("O-1_R-1", false, true),
            ("O-2_R-1", false, true),
            ("O-3_R-1", false, true),
            ("O-4_R-1", false, true),
            ("O-5_R-1", false, true),
            ("O-6_R-1", false, true),
            ("O-7_R-1", false, true),
        ];
        use TreeNodeKind::{Channel, Folder};
        assert_eq!(
            placed(&objects, DECLARED),
            [
                SavedFolder::NoFolder,
                SavedFolder::NoFolder,
                folder(&[(Channel, "CH-1"), (Folder, "PB-3")]),
                folder(&[(Channel, "CH-1"), (Folder, "PB-3")]),
                folder(&[(Channel, "CH-1"), (Folder, "PB-3"), (Folder, "PB-4")]),
                folder(&[(Folder, "PB-9")]),
                SavedFolder::NotInSavedTree,
            ]
        );
    }

    #[test]
    fn module_and_stored_inactive_objects_get_no_folder_but_keep_their_other_places() {
        let objects = [
            ("O-3_R-1", true, true),
            ("O-4_R-1", false, false),
            ("O-1_R-1", true, false),
            ("O-2_R-1", true, true),
        ];
        assert_eq!(
            placed(&objects, DECLARED),
            [
                SavedFolder::ModuleObject,
                SavedFolder::Inactive,
                SavedFolder::NoFolder,
                SavedFolder::NoFolder,
            ]
        );
    }

    #[test]
    fn the_retained_importer_fixtures_yield_no_folder() {
        // The two public trees of the WP-06A characterization, unchanged:
        // their Folder nodes declare no RefId and their objects repeat.
        let objects = [("O-7_R-1", false, true), ("O-9_R-1", false, true)];
        assert_eq!(
            placed(
                &objects,
                r#"<GroupObjectTree><Nodes><Node Type="Channel" RefId="CH-1" GroupObjectInstances="O-7_R-1"><Nodes><Node Type="Folder" GroupObjectInstances="O-9_R-1 O-7_R-1"/></Nodes></Node></Nodes></GroupObjectTree>"#
            ),
            [SavedFolder::Ambiguous, SavedFolder::UnidentifiedNode]
        );
        assert_eq!(
            placed(
                &objects,
                r#"<GroupObjectTree GroupObjectInstances="O-7_R-1 O-7_R-1"><Node Type="Folder" GroupObjectInstances="O-9_R-1"/></GroupObjectTree>"#
            ),
            [SavedFolder::Ambiguous, SavedFolder::UnidentifiedNode]
        );
    }

    #[test]
    fn nodes_without_a_unique_declared_identity_refuse_everything_below_them() {
        let objects = [
            ("O-1_R-1", false, true),
            ("O-2_R-1", false, true),
            ("O-3_R-1", false, true),
            ("O-4_R-1", false, true),
            ("O-5_R-1", false, true),
            ("O-6_R-1", false, true),
        ];
        let tree = r#"<GroupObjectTree><Node Type="Folder" RefId="" GroupObjectInstances="O-1_R-1"><Nodes><Node Type="Folder" RefId="PB-2" GroupObjectInstances="O-2_R-1"/></Nodes></Node><Node Type="ParameterBlock" RefId="PB-3" GroupObjectInstances="O-3_R-1"/><Node Type="Folder" RefId="PB-4" GroupObjectInstances="O-4_R-1"><Nodes><Node Type="Folder" RefId="PB-5" GroupObjectInstances="O-6_R-1"/></Nodes></Node><Node Type="Folder" RefId="PB-4" GroupObjectInstances="O-5_R-1"/></GroupObjectTree>"#;
        assert_eq!(
            placed(&objects, tree),
            vec![SavedFolder::UnidentifiedNode; 6]
        );
    }

    #[test]
    fn unreadable_or_foreign_trees_refuse_every_object() {
        let objects = [("O-1_R-1", false, true)];
        for tree in [
            r#"<GroupObjectTree><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/>"#,
            r#"<GroupObjectTree><Node xmlns="urn:synthetic-extension"/><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#,
            r#"<GroupObjectTree><Nodes><Nodes/></Nodes><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#,
            r#"<GroupObjectTree><k:Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#,
            r#"<GroupObjectTree><Node Type="Folder" RefId="PB-1" RefId="PB-2" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#,
            r#"<Nodes><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></Nodes>"#,
            r#"<!DOCTYPE x><GroupObjectTree><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#,
        ] {
            assert_eq!(
                placed(&objects, tree),
                [SavedFolder::TreeUnresolved],
                "{tree}"
            );
        }
    }

    #[test]
    fn a_declared_project_namespace_prefix_is_read_like_the_inherited_one() {
        let objects = [("O-1_R-1", false, true)];
        let tree = r#"<k:GroupObjectTree xmlns:k="http://knx.org/xml/project/23"><k:Nodes><k:Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></k:Nodes></k:GroupObjectTree>"#;
        assert_eq!(
            placed(&objects, tree),
            [folder(&[(TreeNodeKind::Folder, "PB-1")])]
        );
    }

    #[test]
    fn a_tree_is_only_used_for_exactly_its_own_device_and_source() {
        let tree = r#"<GroupObjectTree><Node Type="Folder" RefId="PB-1" GroupObjectInstances="O-1_R-1"/></GroupObjectTree>"#;
        let mut input = input(&[("O-1_R-1", false, true)]);
        let only = |rows: &[StoredOpaqueEntry], input: &FolderInput| {
            saved_folders(input, rows).into_values().collect::<Vec<_>>()
        };
        // Another device's tree, another source member, another row kind.
        let mut other_kind = tree_row("P-0001/0.xml", "P-0001-0_DI-1", tree);
        other_kind.kind = "SelectiveImportRetainedAttribute".into();
        for rows in [
            vec![tree_row("P-0001/0.xml", "P-0001-0_DI-10", tree)],
            vec![tree_row("P-0002/0.xml", "P-0001-0_DI-1", tree)],
            vec![other_kind],
        ] {
            assert_eq!(only(&rows, &input), [SavedFolder::NotInSavedTree]);
        }
        let mine = tree_row("P-0001/0.xml", "P-0001-0_DI-1", tree);
        assert_eq!(
            only(std::slice::from_ref(&mine), &input),
            [folder(&[(TreeNodeKind::Folder, "PB-1")])]
        );
        // Two matching trees, or two devices claiming the same source.
        assert_eq!(
            only(&[mine.clone(), mine.clone()], &input),
            [SavedFolder::TreeUnresolved]
        );
        input.shared_source = true;
        assert_eq!(
            only(std::slice::from_ref(&mine), &input),
            [SavedFolder::TreeUnresolved]
        );
        input.shared_source = false;
        input.source.ets_id.clear();
        assert_eq!(
            only(std::slice::from_ref(&mine), &input),
            [SavedFolder::NotInSavedTree]
        );
    }
}
