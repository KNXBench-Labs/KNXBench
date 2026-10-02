//! Bounded, deterministic discovery of explicitly configured private product corpora.

#[allow(dead_code)] // Only report-producing test targets need publication helpers.
pub(crate) mod output;

use std::ffi::OsString;
use std::fs;
use std::io::{Cursor, Read};
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustix::fs::{fstat, openat2, Mode, OFlags, ResolveFlags, CWD};

const MAX_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_BUNDLE_DEPTH: usize = 3;
const MAX_FILESYSTEM_DEPTH: usize = 16;
const MAX_DISCOVERY_ENTRIES: usize = 16_384;
const MAX_ARCHIVE_ENTRIES: usize = 16_384;
const MAX_TOTAL_BUNDLE_ENTRIES: usize = 65_536;
const MAX_PACKAGE_SOURCES: usize = 256;
const MAX_CORPUS_BYTES_PER_PASS: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct DiscoveredCorpus {
    pub(crate) packages: Vec<CorpusPackageSource>,
}

#[derive(Debug)]
pub(crate) struct ConfiguredCorpus {
    pub(crate) canonical_root: PathBuf,
    pub(crate) root_device: u64,
    root: Arc<OwnedFd>,
    scope_paths: Option<Vec<PathBuf>>,
}

impl ConfiguredCorpus {
    pub(crate) fn discover(self) -> DiscoveredCorpus {
        let scopes = match self.scope_paths.as_ref() {
            Some(scopes) => scopes.clone(),
            None => {
                let configured_scopes = std::env::var_os("KNXBENCH_PRODUCT_CORPUS_SCOPES")
                    .expect("SKIP: set KNXBENCH_PRODUCT_CORPUS_SCOPES to an explicit path list");
                std::env::split_paths(&configured_scopes).collect::<Vec<_>>()
            }
        };
        assert!(
            !scopes.is_empty(),
            "configured product corpus scope list is empty"
        );
        for scope in &scopes {
            assert!(!scope.as_os_str().is_empty(), "corpus scope is empty");
        }
        let relative_scopes = resolve_scopes(&self.canonical_root, &scopes);
        discover_open_corpus(self, relative_scopes)
    }
}

#[derive(Debug, Default)]
pub(crate) struct CorpusReadBudget {
    consumed: u64,
    archive_entries: usize,
}

impl CorpusReadBudget {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn charge(&mut self, bytes: u64) {
        self.consumed = self
            .consumed
            .checked_add(bytes)
            .expect("corpus read-byte counter overflowed");
        assert!(
            self.consumed <= MAX_CORPUS_BYTES_PER_PASS,
            "corpus decompression exceeds the per-pass byte limit"
        );
    }

    fn charge_archive(&mut self, bytes: &[u8]) {
        let entries = bounded_zip_entry_count(bytes);
        self.archive_entries = self
            .archive_entries
            .checked_add(entries)
            .expect("corpus read archive-entry counter overflowed");
        assert!(
            self.archive_entries <= MAX_TOTAL_BUNDLE_ENTRIES,
            "corpus reads contain too many archive members in total"
        );
    }
}

#[derive(Debug, Default)]
struct DiscoveryBudget {
    filesystem_entries: usize,
    archive_entries: usize,
    package_sources: usize,
    expanded_bytes: u64,
}

impl DiscoveryBudget {
    fn charge_filesystem_entry(&mut self) {
        self.filesystem_entries += 1;
        assert!(
            self.filesystem_entries <= MAX_DISCOVERY_ENTRIES,
            "corpus contains too many filesystem entries"
        );
    }

    fn charge_archive_entries(&mut self, count: usize) {
        self.archive_entries = self
            .archive_entries
            .checked_add(count)
            .expect("corpus archive-entry counter overflowed");
        assert!(
            self.archive_entries <= MAX_TOTAL_BUNDLE_ENTRIES,
            "corpus bundles contain too many members in total"
        );
    }

    fn charge_package_source(&mut self) {
        self.package_sources += 1;
        assert!(
            self.package_sources <= MAX_PACKAGE_SOURCES,
            "corpus contains too many package sources"
        );
    }

    fn charge_expanded_bytes(&mut self, bytes: u64) {
        self.expanded_bytes = self
            .expanded_bytes
            .checked_add(bytes)
            .expect("corpus expanded-byte counter overflowed");
        assert!(
            self.expanded_bytes <= MAX_CORPUS_BYTES_PER_PASS,
            "corpus bundle discovery exceeds the expanded-byte limit"
        );
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CorpusPackageSource {
    root: Arc<OwnedFd>,
    root_file: PathBuf,
    member_indices: Vec<usize>,
    discovery_key: String,
}

impl CorpusPackageSource {
    pub(crate) fn load(&self, budget: &mut CorpusReadBudget) -> Vec<u8> {
        let mut bytes = read_bounded_at(&self.root, &self.root_file, "corpus source");
        budget.charge(bytes.len() as u64);
        for &index in &self.member_indices {
            budget.charge_archive(&bytes);
            let nested = {
                let mut archive = zip::ZipArchive::new(Cursor::new(&bytes))
                    .expect("discovered corpus bundle is no longer a readable ZIP");
                let mut member = archive
                    .by_index(index)
                    .expect("discovered corpus member is no longer readable");
                let size = member.size();
                read_member_bounded(&mut member, size)
            };
            budget.charge(nested.len() as u64);
            bytes = nested;
        }
        bytes
    }
}

pub(crate) fn configured_corpus() -> ConfiguredCorpus {
    let configured_root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .expect("SKIP: set KNXBENCH_PRODUCT_CORPUS to run the private corpus regression");
    open_corpus_root(&configured_root)
}

#[allow(dead_code)] // Used by the matrix test crate's synthetic confinement tests.
pub(crate) fn discover_packages(
    confinement_root: &Path,
    scopes: &[PathBuf],
) -> Vec<CorpusPackageSource> {
    open_corpus(confinement_root, scopes).discover().packages
}

pub(crate) fn open_corpus(confinement_root: &Path, scopes: &[PathBuf]) -> ConfiguredCorpus {
    let mut corpus = open_corpus_root(confinement_root);
    corpus.scope_paths = Some(scopes.to_vec());
    corpus
}

fn open_corpus_root(confinement_root: &Path) -> ConfiguredCorpus {
    let canonical_root = confinement_root
        .canonicalize()
        .expect("configured product corpus is unavailable");
    assert!(
        canonical_root.is_dir(),
        "configured product corpus root is not a directory"
    );
    let root = Arc::new(open_absolute_directory(&canonical_root));
    let root_stat = fstat(&root).expect("read corpus root metadata");
    ConfiguredCorpus {
        canonical_root,
        root_device: root_stat.st_dev,
        root,
        scope_paths: None,
    }
}

fn resolve_scopes(canonical_root: &Path, scopes: &[PathBuf]) -> Vec<PathBuf> {
    scopes
        .iter()
        .map(|scope| {
            let candidate = if scope.is_absolute() {
                scope.clone()
            } else {
                canonical_root.join(scope)
            };
            candidate
                .canonicalize()
                .expect("configured corpus scope is unavailable")
                .strip_prefix(canonical_root)
                .expect("configured corpus scope escapes the corpus root")
                .to_path_buf()
        })
        .collect()
}

fn discover_open_corpus(
    corpus: ConfiguredCorpus,
    relative_scopes: Vec<PathBuf>,
) -> DiscoveredCorpus {
    let ConfiguredCorpus {
        canonical_root,
        root_device,
        root,
        scope_paths: _,
    } = corpus;
    let mut files = Vec::new();
    let mut budget = DiscoveryBudget::default();

    for relative_scope in &relative_scopes {
        let scope_fd = open_directory_beneath(&root, relative_scope);
        visit_directory(&root, &scope_fd, relative_scope, 0, &mut budget, &mut files);
    }
    files.sort();
    files.dedup();

    let mut packages = Vec::new();
    for path in files {
        if has_extension(&path, "knxprod") {
            budget.charge_package_source();
            packages.push(CorpusPackageSource {
                root: Arc::clone(&root),
                discovery_key: path.to_string_lossy().into_owned(),
                root_file: path,
                member_indices: Vec::new(),
            });
        } else if has_extension(&path, "zip") {
            let bytes = read_bounded_at(&root, &path, "corpus bundle");
            budget.charge_expanded_bytes(bytes.len() as u64);
            discover_in_archive(
                &bytes,
                &root,
                &path,
                Vec::new(),
                path.to_string_lossy().into_owned(),
                0,
                &mut budget,
                &mut packages,
            );
        }
    }
    packages.sort_by(|left, right| left.discovery_key.cmp(&right.discovery_key));
    let _ = (canonical_root, root_device);
    DiscoveredCorpus { packages }
}

fn open_absolute_directory(path: &Path) -> OwnedFd {
    openat2(
        CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .expect("open canonical corpus root without following symlinks")
}

fn open_directory_beneath(root: &OwnedFd, path: &Path) -> OwnedFd {
    let relative = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    openat2(
        root,
        relative,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV,
    )
    .expect("open corpus directory beneath the configured root")
}

fn visit_directory(
    root: &Arc<OwnedFd>,
    directory: &OwnedFd,
    relative: &Path,
    depth: usize,
    budget: &mut DiscoveryBudget,
    files: &mut Vec<PathBuf>,
) {
    assert!(
        depth <= MAX_FILESYSTEM_DEPTH,
        "corpus directory nesting exceeds the discovery depth limit"
    );
    budget.charge_filesystem_entry();

    let remaining = MAX_DISCOVERY_ENTRIES - budget.filesystem_entries;
    let directory_path = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
    let mut children = Vec::<(OsString, fs::FileType)>::new();
    for entry in fs::read_dir(directory_path)
        .expect("corpus directory is unreadable")
        .take(remaining + 1)
    {
        let entry = entry.expect("corpus directory entry is unreadable");
        children.push((
            entry.file_name(),
            entry
                .file_type()
                .expect("corpus directory entry type is unreadable"),
        ));
    }
    assert!(
        children.len() <= remaining,
        "corpus contains too many filesystem entries"
    );
    children.sort_by(|left, right| left.0.cmp(&right.0));

    for (name, file_type) in children {
        budget.charge_filesystem_entry();
        assert!(
            !file_type.is_symlink(),
            "corpus entries must not be symbolic links"
        );
        let child_relative = relative.join(&name);
        if file_type.is_dir() {
            let child = open_directory_beneath(root, &child_relative);
            visit_directory(root, &child, &child_relative, depth + 1, budget, files);
        } else if file_type.is_file() {
            files.push(child_relative);
        } else {
            panic!("corpus entry is not a regular file or directory");
        }
    }
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

#[allow(clippy::too_many_arguments)]
fn discover_in_archive(
    bytes: &[u8],
    root: &Arc<OwnedFd>,
    root_file: &Path,
    member_indices: Vec<usize>,
    discovery_prefix: String,
    depth: usize,
    budget: &mut DiscoveryBudget,
    packages: &mut Vec<CorpusPackageSource>,
) {
    assert!(
        depth < MAX_BUNDLE_DEPTH,
        "corpus bundle nesting exceeds the supported discovery depth"
    );
    let entry_count = bounded_zip_entry_count(bytes);
    budget.charge_archive_entries(entry_count);
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).expect("corpus bundle is not a readable ZIP");
    let mut candidates = Vec::new();
    for index in 0..archive.len() {
        let member = archive
            .by_index_raw(index)
            .expect("corpus bundle member metadata is unreadable");
        if member.is_dir() {
            continue;
        }
        let name = member.name().to_string();
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".knxprod") || lower.ends_with(".zip") {
            candidates.push((name, index, lower.ends_with(".knxprod")));
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

    for (name, index, is_package) in candidates {
        let mut indices = member_indices.clone();
        indices.push(index);
        let discovery_key = format!("{discovery_prefix}!{name}");
        if is_package {
            budget.charge_package_source();
            packages.push(CorpusPackageSource {
                root: Arc::clone(root),
                root_file: root_file.to_path_buf(),
                member_indices: indices,
                discovery_key,
            });
            continue;
        }
        let mut member = archive
            .by_index(index)
            .expect("nested corpus bundle is unreadable");
        let size = member.size();
        let nested = read_member_bounded(&mut member, size);
        budget.charge_expanded_bytes(nested.len() as u64);
        discover_in_archive(
            &nested,
            root,
            root_file,
            indices,
            discovery_key,
            depth + 1,
            budget,
            packages,
        );
    }
}

fn read_bounded_at(root: &OwnedFd, path: &Path, kind: &str) -> Vec<u8> {
    let fd = openat2(
        root,
        path,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_XDEV,
    )
    .expect("open corpus source beneath the configured root");
    let metadata = fstat(&fd).expect("opened corpus source metadata is unreadable");
    assert!(
        rustix::fs::FileType::from_raw_mode(metadata.st_mode) == rustix::fs::FileType::RegularFile,
        "corpus source is not a regular file"
    );
    assert!(
        metadata.st_size >= 0 && metadata.st_size as u64 <= MAX_SOURCE_BYTES,
        "{kind} exceeds the discovery size limit"
    );

    let file = fs::File::from(fd);
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .expect("corpus source is unreadable");
    assert!(
        bytes.len() as u64 <= MAX_SOURCE_BYTES,
        "{kind} exceeds the discovery size limit"
    );
    assert_eq!(
        bytes.len() as u64,
        metadata.st_size as u64,
        "corpus source changed while it was read"
    );
    bytes
}

fn read_member_bounded(member: &mut impl Read, declared_size: u64) -> Vec<u8> {
    assert!(
        declared_size <= MAX_SOURCE_BYTES,
        "nested corpus member exceeds the discovery size limit"
    );
    let mut bytes = Vec::new();
    member
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .expect("nested corpus member is unreadable");
    assert!(
        bytes.len() as u64 <= MAX_SOURCE_BYTES,
        "nested corpus member exceeds the discovery size limit"
    );
    assert_eq!(
        bytes.len() as u64,
        declared_size,
        "nested corpus member size changed while reading"
    );
    bytes
}

pub(crate) fn bounded_zip_entry_count(bytes: &[u8]) -> usize {
    const EOCD_SIZE: usize = 22;
    let end = bytes
        .len()
        .checked_sub(EOCD_SIZE)
        .expect("corpus ZIP is shorter than an EOCD record");
    let mut eocd = None;
    for offset in 0..=end {
        if bytes.get(offset..offset + 4) != Some(b"PK\x05\x06") {
            continue;
        }
        assert!(
            eocd.replace(offset).is_none(),
            "corpus ZIP contains an ambiguous fallback EOCD signature"
        );
    }
    let eocd = eocd.expect("corpus ZIP must contain exactly one terminal EOCD record");
    let comment = bytes
        .get(eocd + 20..eocd + 22)
        .map(|raw| u16::from_le_bytes([raw[0], raw[1]]) as usize)
        .expect("corpus ZIP EOCD comment length is missing");
    assert_eq!(
        eocd + EOCD_SIZE + comment,
        bytes.len(),
        "corpus ZIP must contain exactly one terminal EOCD record"
    );
    let record = &bytes[eocd..eocd + EOCD_SIZE];
    let disk = u16::from_le_bytes([record[4], record[5]]);
    let central_disk = u16::from_le_bytes([record[6], record[7]]);
    let disk_entries = u16::from_le_bytes([record[8], record[9]]);
    let total_entries = u16::from_le_bytes([record[10], record[11]]);
    let central_size = u32::from_le_bytes(record[12..16].try_into().unwrap());
    let central_offset = u32::from_le_bytes(record[16..20].try_into().unwrap());
    assert!(
        disk == 0 && central_disk == 0 && disk_entries == total_entries,
        "multi-disk corpus ZIPs are unsupported"
    );
    assert!(
        total_entries != u16::MAX && central_size != u32::MAX && central_offset != u32::MAX,
        "ZIP64 corpus archives are unsupported"
    );
    let central_end = (central_offset as usize)
        .checked_add(central_size as usize)
        .expect("corpus ZIP central-directory range overflowed");
    assert!(
        central_end <= eocd,
        "corpus ZIP central directory extends beyond its EOCD"
    );
    let count = total_entries as usize;
    assert!(
        count <= MAX_ARCHIVE_ENTRIES,
        "corpus archive contains too many members"
    );
    count
}
