//! Communication object flags, object size, and directional group links.
//!
//! No string parsing here — turning ETS's flag and size strings into these
//! types is the importer's job (Session 3); this module only models the
//! resolved shape.

use crate::ids::GroupAddressId;
use crate::provenance::Override;

/// The five communication object flags (`ReadFlag`, `WriteFlag`,
/// `TransmitFlag`, `UpdateFlag`, `CommunicationFlag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComFlags {
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
}

/// A communication object's size, as observed ranging from `"1 Bit"` to
/// `"14 Bytes"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectSize {
    Bit(u8),
    Byte(u16),
}

/// Which direction a communication object uses a group address in. Never
/// flattened into an undirected association — the reference project has 569
/// send links against 27 receive links (DATA_MODEL §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Send,
    Receive,
}

/// A directional link from a communication object instance to a group
/// address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupLink {
    pub ga: GroupAddressId,
    pub direction: Direction,
}

/// The five communication flags, each resolved independently.
///
/// The override chain works per attribute: a `ComObjectInstanceRef` in the
/// reference project sets `ReadFlag` 39 times, `UpdateFlag` 30, `TransmitFlag`
/// 27, `WriteFlag` 18 and `CommunicationFlag` 8 — never all five together. A
/// single `Resolved<ComFlags>` would have to invent the four it was not told
/// about. `ComFlags` remains the fully resolved five-flag view, produced once
/// the product database supplies the program-level defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ResolvedFlags {
    pub read: Override<bool>,
    pub write: Override<bool>,
    pub transmit: Override<bool>,
    pub update: Override<bool>,
    pub communication: Override<bool>,
}

impl ResolvedFlags {
    /// No flag stated at any layer.
    pub fn none() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::{Layer, Override, Resolved};

    #[test]
    fn a_partial_flag_override_leaves_the_other_flags_absent() {
        let flags = ResolvedFlags {
            read: Override::Value(Resolved {
                value: true,
                layer: Layer::Instance,
            }),
            ..ResolvedFlags::none()
        };
        assert_eq!(flags.read.value().map(|r| r.value), Some(true));
        assert!(!flags.write.is_present());
        assert!(!flags.communication.is_present());
    }
}
