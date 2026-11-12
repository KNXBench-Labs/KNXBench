//! Typed server-only comparison of links and effective flags at the last download.

use serde::Serialize;
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DownloadComparisonState {
    Usable,
    Absent,
    OtherProgram,
    ProductUnavailable,
    CurrentUnreadable,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DownloadSection {
    pub state: DownloadComparisonState,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DownloadObjectFlags {
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
    /// Null means this format did not write this flag, never a resolved false.
    pub read_on_init: Option<bool>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DownloadLinkDifference {
    /// Exact raw KNX group address, independent of display style.
    pub group_address: u16,
    /// Presence and sending direction: null means not linked on that side.
    pub downloaded: Option<bool>,
    pub current: Option<bool>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DownloadFlagDifference {
    /// Null means the object was not active on that side of a usable comparison.
    pub downloaded: Option<DownloadObjectFlags>,
    pub current: Option<DownloadObjectFlags>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DownloadObjectDifference {
    pub number: u8,
    pub instance_id: Option<u32>,
    pub links: Vec<DownloadLinkDifference>,
    pub flags: Option<DownloadFlagDifference>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ObjectLastDownload {
    pub links: DownloadSection,
    pub flags: DownloadSection,
    /// Every differing number, including ones with no selected project row.
    pub differing: Vec<DownloadObjectDifference>,
    /// Exactly the selected ComObjectNode.id, not an object number or product reference.
    pub instances: BTreeMap<u32, DownloadObjectDifference>,
    /// A mixed links/flags difference on one number counts once.
    pub differing_objects: usize,
    pub no_selected_row: usize,
}
