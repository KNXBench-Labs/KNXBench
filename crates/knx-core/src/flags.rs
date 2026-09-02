//! Communication object flags, object size, and directional group links.
//!
//! No string parsing here — turning ETS's flag and size strings into these
//! types is the importer's job (Session 3); this module only models the
//! resolved shape.

use crate::ids::GroupAddressId;

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
