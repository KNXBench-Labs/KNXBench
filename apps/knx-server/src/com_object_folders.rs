//! Puts the saved project folder (WP-06a) on each object of a device detail.
//!
//! The folder comes from [`knx_app::com_object_folders`], which reads only the
//! device's retained `GroupObjectTree`. This overlay runs last, after the
//! activation overlay, so one more rule can apply here: an object the current
//! evaluation finds inactive gets no folder either, even when the saved tree
//! (written when it was active) still lists it in one.

use std::collections::BTreeMap;

use knx_app::com_object_folders::{SavedFolder, TreeNodeKind};
use knx_core::ComObjectInstanceId;
use knx_projection::{
    ComObjectActivation, ComObjectProjectFolder, DeviceDetail, ProjectFolderStatus, ProjectTreeNode,
};

/// Sets `project_folder` on every object of `detail` that `folders` places.
pub(crate) fn apply_project_folders(
    detail: &mut DeviceDetail,
    folders: &BTreeMap<ComObjectInstanceId, SavedFolder>,
) {
    for com in &mut detail.com_objects {
        let Some(saved) = folders.get(&ComObjectInstanceId(com.id)) else {
            continue;
        };
        let mut folder = project_folder(saved);
        if folder.status == ProjectFolderStatus::Assigned
            && com.activation == ComObjectActivation::Inactive
        {
            folder = refused(ProjectFolderStatus::Inactive);
        }
        com.project_folder = Some(folder);
    }
}

fn refused(status: ProjectFolderStatus) -> ComObjectProjectFolder {
    ComObjectProjectFolder {
        status,
        key: None,
        path: Vec::new(),
    }
}

fn project_folder(saved: &SavedFolder) -> ComObjectProjectFolder {
    match saved {
        SavedFolder::Folder { path } => {
            let path: Vec<ProjectTreeNode> = path
                .iter()
                .map(|node| ProjectTreeNode {
                    kind: match node.kind {
                        TreeNodeKind::Channel => "Channel",
                        TreeNodeKind::Folder => "Folder",
                    }
                    .to_owned(),
                    ref_id: node.ref_id.clone(),
                    text: node.text.clone(),
                })
                .collect();
            // Length-prefixed, so no `RefId` spelling can make two paths collide.
            let key = path
                .iter()
                .map(|node| format!("{}:{}:{}", node.kind, node.ref_id.len(), node.ref_id))
                .collect::<Vec<_>>()
                .join("/");
            ComObjectProjectFolder {
                status: ProjectFolderStatus::Assigned,
                key: Some(key),
                path,
            }
        }
        SavedFolder::NoFolder => refused(ProjectFolderStatus::NoFolder),
        SavedFolder::NotInSavedTree => refused(ProjectFolderStatus::NotInSavedTree),
        SavedFolder::Ambiguous => refused(ProjectFolderStatus::Ambiguous),
        SavedFolder::UnidentifiedNode => refused(ProjectFolderStatus::UnidentifiedNode),
        SavedFolder::TreeUnresolved => refused(ProjectFolderStatus::TreeUnresolved),
        SavedFolder::ModuleObject => refused(ProjectFolderStatus::ModuleObject),
        SavedFolder::Inactive => refused(ProjectFolderStatus::Inactive),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_app::com_object_folders::TreeNodeRef;

    fn node(kind: TreeNodeKind, ref_id: &str) -> TreeNodeRef {
        TreeNodeRef {
            kind,
            ref_id: ref_id.to_owned(),
            text: None,
        }
    }

    #[test]
    fn keys_follow_the_declared_path_and_cannot_collide_through_spelling() {
        let a = project_folder(&SavedFolder::Folder {
            path: vec![
                node(TreeNodeKind::Channel, "CH-1"),
                node(TreeNodeKind::Folder, "PB-3"),
            ],
        });
        assert_eq!(a.status, ProjectFolderStatus::Assigned);
        assert_eq!(a.key.as_deref(), Some("Channel:4:CH-1/Folder:4:PB-3"));
        assert_eq!(a.path[1].kind, "Folder");
        assert_eq!(a.path[1].ref_id, "PB-3");
        let b = project_folder(&SavedFolder::Folder {
            path: vec![node(TreeNodeKind::Folder, "CH-1/Folder:4:PB-3")],
        });
        assert_ne!(a.key, b.key);
    }

    #[test]
    fn saved_text_changes_neither_opaque_keys_nor_ids_and_is_not_interpolated() {
        let path = vec![
            node(TreeNodeKind::Channel, "CH-1"),
            node(TreeNodeKind::Folder, "PB-3"),
        ];
        let legacy = project_folder(&SavedFolder::Folder { path: path.clone() });
        for text in ["", "   ", " & <b>{{ChNo}}</b> "] {
            let mut captioned = path.clone();
            captioned[0].text = Some("Channel {{0}}".into());
            captioned[1].text = Some(text.into());
            let projected = project_folder(&SavedFolder::Folder { path: captioned });
            assert_eq!(projected.key, legacy.key);
            assert_eq!(
                projected.key.as_deref(),
                Some("Channel:4:CH-1/Folder:4:PB-3")
            );
            assert_eq!(projected.path[0].ref_id, "CH-1");
            assert_eq!(projected.path[0].text.as_deref(), Some("Channel {{0}}"));
            assert_eq!(projected.path[1].ref_id, "PB-3");
            assert_eq!(projected.path[1].text.as_deref(), Some(text));
        }
        assert_eq!(
            serde_json::to_value(&legacy.path[1]).unwrap(),
            serde_json::json!({ "kind": "Folder", "ref_id": "PB-3" })
        );
    }

    #[test]
    fn saved_node_text_json_accepts_legacy_absence_and_preserves_raw_values() {
        let legacy = serde_json::json!({ "kind": "Folder", "ref_id": "PB-1" });
        let node: ProjectTreeNode = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(node.text, None);
        assert_eq!(serde_json::to_value(&node).unwrap(), legacy);
        for text in ["", "   ", " & <b>{{ChNo}}</b> "] {
            let json = serde_json::json!({ "kind": "Folder", "ref_id": "PB-1", "text": text });
            let node: ProjectTreeNode = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(node.text.as_deref(), Some(text));
            assert_eq!(serde_json::to_value(&node).unwrap(), json);
        }
    }

    #[test]
    fn an_evaluated_inactive_object_loses_its_saved_folder() {
        let com = |id: u32, activation: ComObjectActivation| knx_projection::ComObjectNode {
            id,
            number: 0,
            name: None,
            dpt: None,
            dpt_layer: None,
            description: None,
            description_layer: None,
            is_active: true,
            read: false,
            write: false,
            transmit: false,
            update: false,
            communication: false,
            read_on_init: false,
            links: vec![],
            activation,
            channel: None,
            program_dpt: None,
            dpt_text: None,
            dpt_text_language: None,
            size: None,
            function_text: None,
            name_fill: None,
            project_folder: None,
        };
        let mut detail = DeviceDetail {
            id: 1,
            name: String::new(),
            description: None,
            address: None,
            com_objects: vec![
                com(1, ComObjectActivation::Inactive),
                com(2, ComObjectActivation::Undetermined),
                com(3, ComObjectActivation::Inactive),
                com(4, ComObjectActivation::Active),
            ],
            product: knx_projection::DeviceProductNode {
                product_ref: None,
                program_ref: None,
                catalog: None,
                resolution: knx_projection::ProductResolution::NoReference,
            },
        };
        let mut saved_node = node(TreeNodeKind::Folder, "PB-1");
        saved_node.text = Some("Must disappear with inactive placement".into());
        let saved = SavedFolder::Folder {
            path: vec![saved_node],
        };
        let folders = BTreeMap::from([
            (ComObjectInstanceId(1), saved.clone()),
            (ComObjectInstanceId(2), saved.clone()),
            (ComObjectInstanceId(3), SavedFolder::NoFolder),
        ]);
        apply_project_folders(&mut detail, &folders);
        let statuses: Vec<_> = detail
            .com_objects
            .iter()
            .map(|c| c.project_folder.as_ref().map(|f| f.status))
            .collect();
        assert_eq!(
            statuses,
            [
                Some(ProjectFolderStatus::Inactive),
                Some(ProjectFolderStatus::Assigned),
                Some(ProjectFolderStatus::NoFolder),
                None,
            ]
        );
        assert_eq!(
            detail.com_objects[0].project_folder.as_ref().unwrap().key,
            None
        );
        assert!(detail.com_objects[0]
            .project_folder
            .as_ref()
            .unwrap()
            .path
            .is_empty());
    }

    #[test]
    fn refusals_carry_neither_key_nor_path() {
        for (saved, status) in [
            (SavedFolder::NoFolder, ProjectFolderStatus::NoFolder),
            (
                SavedFolder::NotInSavedTree,
                ProjectFolderStatus::NotInSavedTree,
            ),
            (SavedFolder::Ambiguous, ProjectFolderStatus::Ambiguous),
            (
                SavedFolder::UnidentifiedNode,
                ProjectFolderStatus::UnidentifiedNode,
            ),
            (
                SavedFolder::TreeUnresolved,
                ProjectFolderStatus::TreeUnresolved,
            ),
            (SavedFolder::ModuleObject, ProjectFolderStatus::ModuleObject),
            (SavedFolder::Inactive, ProjectFolderStatus::Inactive),
        ] {
            assert_eq!(project_folder(&saved), refused(status));
        }
    }
}
