//! A conservative full snapshot-clone guard, including private Core side maps.
//!
//! The projection's field traversal cannot enumerate orphan ProgramDefaults
//! entries in `Devices`. Derived Debug walks those too without copying them.
//! This sink retains no content (including secrets); it aborts on the first
//! over-budget formatting fragment. The stricter debug-shape byte admission is
//! only for adapter cloning, not a persistence format or public wire schema.
use knx_core::Project;
use std::fmt::{self, Write};

pub(super) fn admit(project: &Project) -> Result<(), String> {
    struct Counter {
        bytes: usize,
    }
    impl Write for Counter {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            self.bytes = self.bytes.checked_add(value.len()).ok_or(fmt::Error)?;
            if self.bytes > 32 * 1024 * 1024 {
                return Err(fmt::Error);
            }
            Ok(())
        }
    }
    let mut sink = Counter { bytes: 0 };
    write!(sink, "{project:?}").map_err(|_| {
        "snapshot clone input limit exceeded (32 MiB conservative normalized debug walk)"
            .to_string()
    })
}
