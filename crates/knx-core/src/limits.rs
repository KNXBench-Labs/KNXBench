//! Size ceilings shared by every layer that reads or retains files (ADR-0142).
//!
//! These are not KNX limits: no KNX specification bounds the size of a
//! product or project file. They are the largest sizes KNXBench can keep.
//! KNX wire-protocol limits are defined where the protocol is implemented.

/// The largest single file, archive member or decoded payload KNXBench reads
/// or retains: 1,000,000,000 bytes. That is SQLite's default
/// `SQLITE_MAX_LENGTH`, the per-value limit of the bundled SQLite every
/// KNXBench store uses (product database, project store, history), so a
/// larger value could be read but never stored. The largest measured real
/// input is a 281.8 MiB application program inside a product package (ADR-0142).
/// A build against an SQLite with a higher limit still refuses above this.
pub const MAX_RETAINED_BYTES: u64 = 1_000_000_000;

/// [`MAX_RETAINED_BYTES`] as a `usize` for in-memory buffers.
pub const MAX_RETAINED_BYTES_USIZE: usize = MAX_RETAINED_BYTES as usize;

/// The largest sum of decoded member sizes one archive may expand to by
/// default: 4 GiB. No format bounds it (a ZIP member's size is independent of
/// its siblings'); it guards against decompression bombs. The largest measured
/// real product package expands to 1.7 GiB, the largest project archive far
/// less (ADR-0142).
pub const MAX_EXPANDED_ARCHIVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ceilings_are_ordered() {
        const { assert!(MAX_RETAINED_BYTES < MAX_EXPANDED_ARCHIVE_BYTES) };
        // The largest measured real program (281.8 MiB) fits with room.
        const { assert!(MAX_RETAINED_BYTES > 3 * 281 * 1024 * 1024) };
    }
}
