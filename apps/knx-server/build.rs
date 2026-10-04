//! Stamps the short git commit into the build so `--version` can name it.
//!
//! All logic lives in `knx-build-stamp` (shared by both binaries): an
//! explicit `KNX_BUILD_SHA`, else this workspace's `HEAD`, else nothing —
//! and with `KNX_REQUIRE_CLEAN_TREE=1` a release build fails instead of
//! naming a commit its tree differs from.

fn main() {
    knx_build_stamp::emit();
}
