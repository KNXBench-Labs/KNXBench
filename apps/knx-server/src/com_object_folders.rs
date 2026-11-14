//! Puts the saved project folder (WP-06a) on each object of a device detail.
//!
//! The folder comes from [`knx_app::com_object_folders`], which reads only the
//! device's retained `GroupObjectTree`. This overlay runs last, after the
//! activation overlay, so one more rule can apply here: an object the current
//! evaluation finds inactive gets no folder either, even when the saved tree
//! (written when it was active) still lists it in one.

use std::collections::BTreeMap;

use knx_app::com_object_folders::{SavedFolder, TreeNodeKind};
use knx_app::saved_folder_currency::SavedTreeCurrency;
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

/// WP-06: says whether the saved folders still describe the device
/// (`docs/contracts/saved-folder-currency.md`), only when an object still shows
/// a saved Folder after the inactive refusal. Never changes a placement.
pub(crate) fn apply_folder_currency(detail: &mut DeviceDetail, currency: SavedTreeCurrency) {
    use knx_app::saved_folder_currency::CurrencyUnknown as U;
    use knx_projection::{ProjectFolderCurrency as C, ProjectFolderCurrencyUnknown as R};
    let shown = detail.com_objects.iter().any(|com| {
        com.project_folder
            .as_ref()
            .is_some_and(|folder| folder.status == ProjectFolderStatus::Assigned)
    });
    if !shown {
        return;
    }
    detail.project_folder_currency = Some(match currency {
        SavedTreeCurrency::Unchanged => C::Unchanged,
        SavedTreeCurrency::ChangedSinceImport => C::ChangedSinceImport,
        SavedTreeCurrency::Unknown(reason) => C::Unknown {
            reason: match reason {
                U::NoRetainedSource => R::NoRetainedSource,
                U::AmbiguousSource => R::AmbiguousSource,
                U::SourceIntegrity => R::SourceIntegrity,
                U::SourceLimit => R::SourceLimit,
                U::UnsupportedSchema => R::UnsupportedSchema,
                U::Unreadable => R::Unreadable,
                U::DeviceNotInSource => R::DeviceNotInSource,
                U::AmbiguousDevice => R::AmbiguousDevice,
            },
        },
    });
}

/// Adds optional declarations only after saved placement and final inactive refusal.
/// Two passes admit every repeated DTO string before applying any caption.
pub(crate) fn apply_product_captions(
    detail: &mut DeviceDetail,
    captions: &knx_productdb::com_object_folder_captions::FolderCaptions,
) -> Result<(), knx_productdb::dynamic::TextProjectionError> {
    let mut budget = knx_productdb::dynamic::TextProjectionBudget::default();
    // Charge the retained source map as well as every eventual DTO copy.
    for caption in captions.values() {
        for text in [
            caption.text.as_str(),
            &caption.application_program_id,
            &caption.parameter_block_id,
            &caption.source_sha256,
            "ProductDeclaration",
            translated_language(caption),
        ] {
            budget.admit_copy(text)?;
        }
    }
    for com in &detail.com_objects {
        let Some(folder) = &com.project_folder else {
            continue;
        };
        if folder.status != ProjectFolderStatus::Assigned {
            continue;
        }
        let path = caption_path(&folder.path);
        for (index, node) in folder.path.iter().enumerate() {
            if node.kind != "Folder" {
                continue;
            }
            if let Some(caption) = captions.get(&path[..=index]) {
                for text in [
                    caption.text.as_str(),
                    &caption.application_program_id,
                    &caption.parameter_block_id,
                    &caption.source_sha256,
                    "ProductDeclaration",
                    translated_language(caption),
                ] {
                    budget.admit_copy(text)?;
                }
            }
        }
    }
    for com in &mut detail.com_objects {
        let Some(folder) = &mut com.project_folder else {
            continue;
        };
        if folder.status != ProjectFolderStatus::Assigned {
            continue;
        }
        let path = caption_path(&folder.path);
        for (index, node) in folder.path.iter_mut().enumerate() {
            if node.kind != "Folder" {
                continue;
            }
            if let Some(caption) = captions.get(&path[..=index]) {
                node.product_caption = Some(knx_projection::ProductFolderCaption {
                    source: knx_projection::ProductFolderCaptionSource::ProductDeclaration,
                    text: caption.text.clone(),
                    application_program_id: caption.application_program_id.clone(),
                    parameter_block_id: caption.parameter_block_id.clone(),
                    source_sha256: caption.source_sha256.clone(),
                    language: caption.language.as_ref().map(caption_language),
                });
            }
        }
    }
    Ok(())
}

/// The selected language identifier a caption copies into the DTO, else empty.
fn translated_language(caption: &knx_productdb::com_object_folder_captions::FolderCaption) -> &str {
    caption
        .language
        .as_ref()
        .and_then(knx_productdb::com_object_folder_captions::CaptionLanguage::translated)
        .unwrap_or_default()
}

/// Maps the reader's language provenance onto the DTO; never feeds ids or keys.
fn caption_language(
    language: &knx_productdb::com_object_folder_captions::CaptionLanguage,
) -> knx_projection::ProductFolderCaptionLanguage {
    use knx_productdb::com_object_folder_captions::{CaptionLanguage, Untranslated};
    use knx_projection::{
        ProductFolderCaptionLanguage as Language, ProductFolderCaptionUntranslated as Reason,
    };
    match language {
        CaptionLanguage::Translated(language) => Language::Translated {
            language: language.clone(),
        },
        CaptionLanguage::Untranslated(reason) => Language::Untranslated {
            reason: match reason {
                Untranslated::NoTranslation => Reason::NoTranslation,
                Untranslated::UnusableTranslation => Reason::UnusableTranslation,
                Untranslated::TranslationDisagrees => Reason::TranslationDisagrees,
            },
        },
    }
}

fn caption_path(
    path: &[ProjectTreeNode],
) -> Vec<knx_productdb::com_object_folder_captions::FolderPathNode> {
    path.iter()
        .map(
            |n| knx_productdb::com_object_folder_captions::FolderPathNode {
                kind: n.kind.clone(),
                ref_id: n.ref_id.clone(),
            },
        )
        .collect()
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
                    product_caption: None,
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
            saved_instance_priority: Default::default(),
            effective_priority: Default::default(),
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
            project_folder_currency: None,
            last_download: None,
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
        // Currency is reported only while an Assigned folder is still shown.
        let mut hidden = detail.clone();
        for com in &mut hidden.com_objects {
            if com
                .project_folder
                .as_ref()
                .is_some_and(|f| f.status == ProjectFolderStatus::Assigned)
            {
                com.project_folder = Some(refused(ProjectFolderStatus::Inactive));
            }
        }
        apply_folder_currency(&mut hidden, SavedTreeCurrency::ChangedSinceImport);
        assert_eq!(hidden.project_folder_currency, None);
        let mut shown = detail.clone();
        apply_folder_currency(&mut shown, SavedTreeCurrency::ChangedSinceImport);
        assert_eq!(
            shown.project_folder_currency,
            Some(knx_projection::ProjectFolderCurrency::ChangedSinceImport)
        );
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
        use knx_productdb::com_object_folder_captions::{FolderCaption, FolderPathNode};
        let captions = BTreeMap::from([(
            vec![FolderPathNode {
                kind: "Folder".into(),
                ref_id: "PB-1".into(),
            }],
            FolderCaption {
                text: "Installed & <b>literal</b>".into(),
                application_program_id: "AP".into(),
                parameter_block_id: "AP_PB-1".into(),
                source_sha256: "a".repeat(64),
                language: None,
            },
        )]);
        let before = serde_json::to_value(&detail).unwrap();
        apply_product_captions(&mut detail, &captions).unwrap();
        let path = &detail.com_objects[1].project_folder.as_ref().unwrap().path;
        assert_eq!(
            path[0].text.as_deref(),
            Some("Must disappear with inactive placement")
        );
        let json = serde_json::to_value(&path[0]).unwrap();
        assert_eq!(
            json["product_caption"],
            serde_json::json!({
                "source": "ProductDeclaration", "text": "Installed & <b>literal</b>",
                "application_program_id": "AP", "parameter_block_id": "AP_PB-1", "source_sha256": "a".repeat(64),
            })
        );
        detail.com_objects[1].project_folder.as_mut().unwrap().path[0].product_caption = None;
        assert_eq!(serde_json::to_value(&detail).unwrap(), before);
        let copies = knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK / 4096 / 2;
        detail.com_objects = vec![detail.com_objects[1].clone(); copies];
        let mut oversized = captions;
        let caption = oversized.values_mut().next().unwrap();
        caption.text = "x".repeat(4096);
        // Repeated metadata, not just the rendered text, must consume the budget.
        caption.application_program_id = "p".repeat(4096);
        caption.parameter_block_id = "b".repeat(4096);
        let before = serde_json::to_value(&detail).unwrap();
        assert!(apply_product_captions(&mut detail, &oversized).is_err());
        assert_eq!(
            serde_json::to_value(&detail).unwrap(),
            before,
            "no partial overlay"
        );
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
