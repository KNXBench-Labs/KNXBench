//! Stamps the short git commit into the build so `--version` can name it.
//!
//! All logic lives in `knx-build-stamp`, shared with the other binaries.

fn main() {
    knx_build_stamp::emit();
}
