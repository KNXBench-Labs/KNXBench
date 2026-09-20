//! Communication object flags, object size, and directional group links.
//!
//! No string parsing here — turning ETS's flag and size strings into these
//! types is the importer's job (Session 3); this module only models the
//! resolved shape.

use crate::ids::GroupAddressId;
use crate::provenance::Override;

/// The six communication object flags (`ReadFlag`, `WriteFlag`,
/// `TransmitFlag`, `UpdateFlag`, `CommunicationFlag`, `ReadOnInitFlag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComFlags {
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
    /// Read-on-Init: the object asks the bus for its group address's value
    /// once, at start-up. Stated by the application program
    /// (`ComObject/@ReadOnInitFlag`), never — so far measured — by a
    /// `ComObjectInstanceRef` (KNOWN_LIMITATIONS §117).
    pub read_on_init: bool,
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

/// The six communication flags, each resolved independently.
///
/// The override chain works per attribute: a `ComObjectInstanceRef` in the
/// reference project sets `ReadFlag` 39 times, `UpdateFlag` 30, `TransmitFlag`
/// 27, `WriteFlag` 18 and `CommunicationFlag` 8 — never all five together, and
/// `ReadOnInitFlag` not once. A single `Resolved<ComFlags>` would have to
/// invent the five it was not told about. `ComFlags` remains the fully
/// resolved six-flag view, produced once the product database supplies the
/// program-level defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ResolvedFlags {
    pub read: Override<bool>,
    pub write: Override<bool>,
    pub transmit: Override<bool>,
    pub update: Override<bool>,
    pub communication: Override<bool>,
    /// Absent in every project file measured so far: the attribute is a
    /// program-layer one, so this is normally filled by `knx-productdb`'s
    /// enrichment at `Layer::Program`/`Layer::ProgramRef`, or by the user at
    /// `Layer::UserEdit`. "Absent" and "stated false" stay distinct here, as
    /// everywhere else in `Override`.
    pub read_on_init: Override<bool>,
}

impl ResolvedFlags {
    /// No flag stated at any layer.
    pub fn none() -> Self {
        Self::default()
    }
}

/// Which of a communication object's six flags a `Command::SetComObjectFlag`
/// targets. Mirrors `ResolvedFlags`' own field order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComFlagKind {
    Read,
    Write,
    Transmit,
    Update,
    Communication,
    ReadOnInit,
}

impl ResolvedFlags {
    /// Borrows the one field `kind` names — lets `Command::SetComObjectFlag`
    /// stay generic over which of the six flags it edits instead of six
    /// near-identical match arms living in `command.rs`.
    pub fn get(&self, kind: ComFlagKind) -> &Override<bool> {
        match kind {
            ComFlagKind::Read => &self.read,
            ComFlagKind::Write => &self.write,
            ComFlagKind::Transmit => &self.transmit,
            ComFlagKind::Update => &self.update,
            ComFlagKind::Communication => &self.communication,
            ComFlagKind::ReadOnInit => &self.read_on_init,
        }
    }

    /// The mutable counterpart of `get`.
    pub fn get_mut(&mut self, kind: ComFlagKind) -> &mut Override<bool> {
        match kind {
            ComFlagKind::Read => &mut self.read,
            ComFlagKind::Write => &mut self.write,
            ComFlagKind::Transmit => &mut self.transmit,
            ComFlagKind::Update => &mut self.update,
            ComFlagKind::Communication => &mut self.communication,
            ComFlagKind::ReadOnInit => &mut self.read_on_init,
        }
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

    #[test]
    fn get_and_get_mut_address_the_matching_field() {
        let mut flags = ResolvedFlags::none();
        *flags.get_mut(ComFlagKind::Communication) = Override::Value(Resolved {
            value: true,
            layer: Layer::UserEdit,
        });
        assert_eq!(
            flags
                .get(ComFlagKind::Communication)
                .value()
                .map(|r| r.value),
            Some(true)
        );
        // Untouched fields stay absent — `get_mut` must not alias another
        // field.
        assert!(!flags.get(ComFlagKind::Read).is_present());
    }

    #[test]
    fn read_on_init_is_a_peer_of_the_other_five() {
        let mut flags = ResolvedFlags::none();
        assert!(!flags.get(ComFlagKind::ReadOnInit).is_present());
        *flags.get_mut(ComFlagKind::ReadOnInit) = Override::Value(Resolved {
            value: true,
            layer: Layer::Program,
        });
        assert_eq!(
            flags.get(ComFlagKind::ReadOnInit).value().map(|r| r.value),
            Some(true)
        );
        assert_eq!(
            flags.read_on_init.value().map(|r| r.layer),
            Some(Layer::Program)
        );
        // The sixth flag is its own field, not an alias of the fifth.
        assert!(!flags.get(ComFlagKind::Communication).is_present());
    }
}
