//! Publishes private-corpus aggregate reports outside the input filesystem.
//!
//! Holds the output directory descriptor across cleanup and atomic rename.
//! Extracted unchanged from the tested matrix publisher; no corpus bytes or
//! identities are published by this filesystem helper.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::fd::OwnedFd;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rustix::fs::{
    fstat, openat, openat2, renameat, unlinkat, AtFlags, Mode, OFlags, ResolveFlags, CWD,
};
use serde_json::Value;

static NEXT_OUTPUT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(crate) struct OutputTarget {
    directory: OwnedFd,
    file_name: OsString,
}

struct PendingOutput<'a> {
    directory: &'a OwnedFd,
    name: OsString,
    published: bool,
}

impl Drop for PendingOutput<'_> {
    fn drop(&mut self) {
        if !self.published {
            let _ = unlinkat(self.directory, &self.name, AtFlags::empty());
        }
    }
}

pub(crate) fn validate_output_path(
    corpus_root: &Path,
    corpus_device: u64,
    path: &Path,
) -> OutputTarget {
    let file_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .expect("matrix output must name a file");
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let canonical_parent = parent
        .canonicalize()
        .expect("matrix output directory must already exist");
    assert!(
        !canonical_parent.starts_with(corpus_root),
        "matrix output must be outside the configured corpus root"
    );
    let directory = openat2(
        CWD,
        &canonical_parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .expect("open matrix output directory without following symlinks");
    let output_device = fstat(&directory)
        .expect("read matrix output directory metadata")
        .st_dev;
    assert_ne!(
        output_device, corpus_device,
        "matrix output directory must be on a different filesystem from the corpus"
    );
    OutputTarget {
        directory,
        file_name: file_name.to_os_string(),
    }
}

pub(crate) fn write_atomic(target: &OutputTarget, value: &Value) {
    let (fd, temp_name) = (0..128)
        .find_map(|_| {
            let sequence = NEXT_OUTPUT_TEMP.fetch_add(1, Ordering::Relaxed);
            let name = OsString::from(format!(
                ".knxbench-matrix-{}-{sequence}.tmp",
                std::process::id()
            ));
            match openat(
                &target.directory,
                &name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(fd) => Some((fd, name)),
                Err(rustix::io::Errno::EXIST) => None,
                Err(error) => panic!("create matrix output file: {error}"),
            }
        })
        .expect("create a unique matrix output file");
    let mut pending = PendingOutput {
        directory: &target.directory,
        name: temp_name,
        published: false,
    };
    let mut output = fs::File::from(fd);
    serde_json::to_writer_pretty(&mut output, value).expect("serialize compatibility matrix");
    output
        .write_all(b"\n")
        .expect("finish compatibility matrix");
    output.flush().expect("flush compatibility matrix");
    renameat(
        &target.directory,
        &pending.name,
        &target.directory,
        &target.file_name,
    )
    .expect("publish compatibility matrix atomically");
    pending.published = true;
}

pub(crate) fn remove_previous_output(target: &OutputTarget) {
    match unlinkat(&target.directory, &target.file_name, AtFlags::empty()) {
        Ok(()) | Err(rustix::io::Errno::NOENT) => {}
        Err(error) => panic!("remove previous matrix output: {error}"),
    }
}
