//! Versioned typed encoding of one complete rehearsal install result.
//!
//! The *complete preview identity* is a deterministic, lossless SHA-256
//! stream over every admitted field of one `ProductFileInstall` — the
//! install-result identity the future preview orchestration computes and that
//! apply compares for exact preview/report agreement. It is distinct from the
//! catalogue witness (which streams SQLite rows) and from the source identity
//! (which binds the source bytes). Construction-only — no production route is
//! wired here. See `docs/contracts/catalogue-jobs/managed-binding.md` §4/§5
//! and `preview-identity-increment.md`.

use sha2::{Digest, Sha256};

use crate::baggage::{
    BaggageInventory, BaggagePayload, MediaClass, NestedArchive, Resolution, ResolvedDeclaration,
};
use crate::package::{
    InstallCategory, InstallDiagnostic, InstallDiagnosticKind, InstallDisposition, InstallFacts,
    InstallReport, PackageMember,
};
use crate::parse::baggage::BaggageDeclaration;
use crate::report::{IdConflict, TranslationCounts, UnknownConstruct, UnknownKind};
use crate::wrapper::{
    InstalledPackage, ProductFileInstall, WrapperContents, WrapperMember, WrapperMemberKind,
};

/// Format version of the preview identity encoding. Bumped only when a later
/// version changes the field set, ordering or equality semantics.
const PREVIEW_IDENTITY_VERSION: u32 = 1;

/// Fixed ASCII domain plus the NUL terminator; the u32 BE version follows.
const DOMAIN: &[u8] = b"KNXBench/PreviewIdentity/1\0";

/// The single tag byte for an absent `Option` value (explicit null).
const TAG_NONE: u8 = 0x00;
/// The single tag byte for a present `Option` value (explicit some).
const TAG_SOME: u8 = 0x01;

/// One complete preview identity: a digest plus exact scan accounting.
///
/// `sha256` is the equality discriminator; `package_count`,
/// `other_member_count` and `encoded_bytes` are exact bookkeeping of the
/// scanned result, never a redacted projection or a partial summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreviewIdentity {
    pub(crate) version: u32,
    pub(crate) sha256: [u8; 32],
    pub(crate) package_count: u32,
    pub(crate) other_member_count: u32,
    pub(crate) encoded_bytes: u64,
}

/// Typed failures of the preview identity encoding; never a string and never
/// a partial digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewIdentityError {
    /// A length or count could not be represented in the fixed-width codec.
    Overflow,
}

impl std::fmt::Display for PreviewIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewIdentityError::Overflow => {
                write!(f, "preview identity encoding overflow")
            }
        }
    }
}

impl std::error::Error for PreviewIdentityError {}

/// Computes the complete preview identity of one install result.
///
/// Deterministic: identical install results encode to identical digests. The
/// stream is complete-or-refused: any field that cannot be represented (a
/// count exceeding `u64`, a `Vec` whose length overflows) returns
/// [`PreviewIdentityError::Overflow`] instead of omitting the field.
pub(crate) fn preview_identity(
    install: &ProductFileInstall,
) -> Result<PreviewIdentity, PreviewIdentityError> {
    let mut encoder = Encoder::default();
    encoder.raw(DOMAIN)?;
    encoder.u32(PREVIEW_IDENTITY_VERSION)?;
    encode_product_file_install(&mut encoder, install)?;
    let (sha256, encoded_bytes) = encoder.finish();

    let package_count = len_u32(install.packages.len())?;
    let other_member_count = match &install.wrapper {
        None => 0,
        Some(wrapper) => len_u32(wrapper.other_members.len())?,
    };
    Ok(PreviewIdentity {
        version: PREVIEW_IDENTITY_VERSION,
        sha256,
        package_count,
        other_member_count,
        encoded_bytes,
    })
}

fn len_u32(len: usize) -> Result<u32, PreviewIdentityError> {
    u32::try_from(len).map_err(|_| PreviewIdentityError::Overflow)
}

fn len_u64(len: usize) -> Result<u64, PreviewIdentityError> {
    u64::try_from(len).map_err(|_| PreviewIdentityError::Overflow)
}

/// A `usize` counter encoded as a checked u64 (the supported targets are
/// 64-bit, so `usize` fits; the check keeps the stream portable and honest).
fn count(v: usize) -> Result<u64, PreviewIdentityError> {
    u64::try_from(v).map_err(|_| PreviewIdentityError::Overflow)
}

/// A checked, big-endian streaming encoder with a single SHA-256 sink. Every
/// byte written is counted; `encoded_bytes` is the external stream byte count.
#[derive(Default)]
struct Encoder {
    hasher: Sha256,
    bytes: u64,
}

impl Encoder {
    fn raw(&mut self, data: &[u8]) -> Result<(), PreviewIdentityError> {
        self.bytes = self
            .bytes
            .checked_add(len_u64(data.len())?)
            .ok_or(PreviewIdentityError::Overflow)?;
        self.hasher.update(data);
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), PreviewIdentityError> {
        self.raw(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), PreviewIdentityError> {
        self.raw(&value.to_be_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PreviewIdentityError> {
        self.raw(&value.to_be_bytes())
    }

    /// A single tag byte: a boolean (0/1) or an `Option` none/some marker.
    fn tag(&mut self, value: u8) -> Result<(), PreviewIdentityError> {
        self.u8(value)
    }

    fn bool(&mut self, value: bool) -> Result<(), PreviewIdentityError> {
        self.tag(u8::from(value))
    }

    /// A length-framed byte string (`u64` byte length then exactly the bytes).
    fn string(&mut self, value: &str) -> Result<(), PreviewIdentityError> {
        self.u64(len_u64(value.len())?)?;
        self.raw(value.as_bytes())
    }

    /// An explicit null/empty distinction: `None` is `TAG_NONE`; `Some("")`
    /// is `TAG_SOME` followed by a zero-length string.
    fn option_string(&mut self, value: Option<&str>) -> Result<(), PreviewIdentityError> {
        match value {
            None => self.tag(TAG_NONE),
            Some(text) => {
                self.tag(TAG_SOME)?;
                self.string(text)
            }
        }
    }

    fn finish(self) -> ([u8; 32], u64) {
        (self.hasher.finalize().into(), self.bytes)
    }
}

fn encode_product_file_install(
    encoder: &mut Encoder,
    install: &ProductFileInstall,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&install.sha256)?;
    match &install.wrapper {
        None => encoder.tag(TAG_NONE)?,
        Some(wrapper) => {
            encoder.tag(TAG_SOME)?;
            encode_wrapper_contents(encoder, wrapper)?;
        }
    }
    encoder.u64(len_u64(install.packages.len())?)?;
    for package in &install.packages {
        encode_installed_package(encoder, package)?;
    }
    Ok(())
}

fn encode_wrapper_contents(
    encoder: &mut Encoder,
    wrapper: &WrapperContents,
) -> Result<(), PreviewIdentityError> {
    encoder.u64(len_u64(wrapper.other_members.len())?)?;
    for member in &wrapper.other_members {
        encode_wrapper_member(encoder, member)?;
    }
    Ok(())
}

fn encode_wrapper_member(
    encoder: &mut Encoder,
    member: &WrapperMember,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&member.path)?;
    encoder.u64(member.size)?;
    encoder.string(&member.sha256)?;
    encoder.u8(match member.kind {
        WrapperMemberKind::File => 0,
        WrapperMemberKind::AppleDouble => 1,
    })
}

fn encode_installed_package(
    encoder: &mut Encoder,
    package: &InstalledPackage,
) -> Result<(), PreviewIdentityError> {
    encoder.option_string(package.path.as_deref())?;
    encode_install_report(encoder, &package.report)
}

fn encode_install_report(
    encoder: &mut Encoder,
    report: &InstallReport,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&report.sha256)?;
    encoder.u32(report.scheme)?;
    encoder.bool(report.skipped)?;
    encoder.u64(len_u64(report.members.len())?)?;
    for member in &report.members {
        encode_package_member(encoder, member)?;
    }
    encoder.u64(count(report.unknown)?)?;
    encoder.u64(len_u64(report.conflicts.len())?)?;
    for conflict in &report.conflicts {
        encode_id_conflict(encoder, conflict)?;
    }
    encode_translation_counts(encoder, &report.translations)?;
    encoder.u64(count(report.dropped_datapoint_types)?)?;
    match &report.facts {
        None => encoder.tag(TAG_NONE)?,
        Some(facts) => {
            encoder.tag(TAG_SOME)?;
            encode_install_facts(encoder, facts)?;
        }
    }
    match &report.baggage {
        None => encoder.tag(TAG_NONE)?,
        Some(baggage) => {
            encoder.tag(TAG_SOME)?;
            encode_baggage_inventory(encoder, baggage)?;
        }
    }
    encoder.u64(len_u64(report.source_names.len())?)?;
    for name in &report.source_names {
        encoder.string(name)?;
    }
    Ok(())
}

fn encode_package_member(
    encoder: &mut Encoder,
    member: &PackageMember,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&member.path)?;
    encoder.string(&member.role)?;
    encoder.string(&member.sha256)?;
    encoder.u64(member.size)
}

fn encode_id_conflict(
    encoder: &mut Encoder,
    conflict: &IdConflict,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&conflict.table)?;
    encoder.string(&conflict.id)?;
    encoder.string(&conflict.kept_sha256)?;
    encoder.string(&conflict.other_sha256)?;
    encoder.u64(u64::from(conflict.occurrence))
}

fn encode_translation_counts(
    encoder: &mut Encoder,
    translations: &TranslationCounts,
) -> Result<(), PreviewIdentityError> {
    encoder.u64(count(translations.program)?)?;
    encoder.u64(count(translations.catalog)?)?;
    encoder.u64(count(translations.hardware)?)?;
    encoder.u64(count(translations.master)?)
}

fn encode_install_facts(
    encoder: &mut Encoder,
    facts: &InstallFacts,
) -> Result<(), PreviewIdentityError> {
    encoder.u64(len_u64(facts.counts.len())?)?;
    for entry in &facts.counts {
        encoder.u8(match entry.category {
            InstallCategory::ArchiveMember => 0,
            InstallCategory::Product => 1,
            InstallCategory::ApplicationProgram => 2,
            InstallCategory::Parameter => 3,
            InstallCategory::CommunicationObject => 4,
            InstallCategory::DynamicNode => 5,
            InstallCategory::Module => 6,
            InstallCategory::BaggageIndex => 7,
            InstallCategory::Baggage => 8,
            InstallCategory::UnknownConstruct => 9,
            InstallCategory::MasterSection => 10,
            InstallCategory::MasterSubtree => 11,
            InstallCategory::DatapointType => 12,
        })?;
        encoder.u8(match entry.disposition {
            InstallDisposition::Read => 0,
            InstallDisposition::Stored => 1,
            InstallDisposition::Deduplicated => 2,
            InstallDisposition::RetainedButUninterpreted => 3,
            InstallDisposition::Unsupported => 4,
            InstallDisposition::Dropped => 5,
        })?;
        encoder.u64(entry.count)?;
    }
    encoder.u64(len_u64(facts.unknown_constructs.len())?)?;
    for unknown in &facts.unknown_constructs {
        encode_unknown_construct(encoder, unknown)?;
    }
    encoder.u64(facts.unknown_occurrences)?;
    encoder.u64(len_u64(facts.diagnostics.len())?)?;
    for diagnostic in &facts.diagnostics {
        encode_install_diagnostic(encoder, diagnostic)?;
    }
    Ok(())
}

fn encode_unknown_construct(
    encoder: &mut Encoder,
    unknown: &UnknownConstruct,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&unknown.xpath)?;
    encoder.u8(match unknown.kind {
        UnknownKind::Element => 0,
        UnknownKind::Attribute => 1,
    })?;
    encoder.string(&unknown.name)?;
    encoder.u64(u64::from(unknown.occurrences))?;
    encoder.option_string(unknown.sample.as_deref())
}

fn encode_install_diagnostic(
    encoder: &mut Encoder,
    diagnostic: &InstallDiagnostic,
) -> Result<(), PreviewIdentityError> {
    encoder.u8(match diagnostic.kind() {
        InstallDiagnosticKind::MasterIdentityCollision => 0,
        InstallDiagnosticKind::MalformedMasterDeclaration => 1,
        InstallDiagnosticKind::UnsupportedMasterSection => 2,
        InstallDiagnosticKind::UnsupportedMasterSubtree => 3,
        InstallDiagnosticKind::UnresolvedBaggageDeclaration => 4,
        InstallDiagnosticKind::UndeclaredBaggagePayload => 5,
    })?;
    encoder.string(diagnostic.archive_path())?;
    encoder.string(diagnostic.xml_path())?;
    encoder.string(diagnostic.detail())?;
    encoder.u64(diagnostic.occurrences())
}

fn encode_baggage_inventory(
    encoder: &mut Encoder,
    baggage: &BaggageInventory,
) -> Result<(), PreviewIdentityError> {
    encoder.u64(len_u64(baggage.declarations.len())?)?;
    for declaration in &baggage.declarations {
        encode_resolved_declaration(encoder, declaration)?;
    }
    encoder.u64(len_u64(baggage.payloads.len())?)?;
    for payload in &baggage.payloads {
        encode_baggage_payload(encoder, payload)?;
    }
    Ok(())
}

fn encode_resolved_declaration(
    encoder: &mut Encoder,
    declaration: &ResolvedDeclaration,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&declaration.index_path)?;
    encoder.u64(declaration.ordinal)?;
    encode_baggage_declaration(encoder, &declaration.declaration)?;
    encoder.u8(match declaration.resolution {
        Resolution::Resolved => 0,
        Resolution::Missing => 1,
        Resolution::Invalid => 2,
    })?;
    encoder.option_string(declaration.member_path.as_deref())?;
    encoder.option_string(declaration.detail.as_deref())
}

fn encode_baggage_declaration(
    encoder: &mut Encoder,
    declaration: &BaggageDeclaration,
) -> Result<(), PreviewIdentityError> {
    encoder.option_string(declaration.id.as_deref())?;
    encoder.option_string(declaration.name.as_deref())?;
    encoder.option_string(declaration.target_path.as_deref())?;
    encoder.option_string(declaration.install_on_import.as_deref())?;
    encoder.option_string(declaration.time_info.as_deref())?;
    encoder.option_string(declaration.file_version.as_deref())
}

fn encode_baggage_payload(
    encoder: &mut Encoder,
    payload: &BaggagePayload,
) -> Result<(), PreviewIdentityError> {
    encoder.string(&payload.member_path)?;
    encoder.string(&payload.sha256)?;
    encoder.u64(payload.size)?;
    encoder.u8(match payload.media_class {
        MediaClass::Empty => 0,
        MediaClass::Png => 1,
        MediaClass::Jpeg => 2,
        MediaClass::Gif => 3,
        MediaClass::Bmp => 4,
        MediaClass::Pdf => 5,
        MediaClass::Zip => 6,
        MediaClass::PeExecutable => 7,
        MediaClass::Ole2Compound => 8,
        MediaClass::Xml => 9,
        MediaClass::Unknown => 10,
    })?;
    match payload.nested {
        NestedArchive::NotArchive => encoder.u8(0)?,
        NestedArchive::Unreadable => encoder.u8(1)?,
        NestedArchive::Read {
            entries,
            declared_expanded_size,
            encrypted_entries,
            archive_named_entries,
        } => {
            encoder.u8(2)?;
            encoder.u64(entries)?;
            encoder.u64(declared_expanded_size)?;
            encoder.u64(encrypted_entries)?;
            encoder.u64(archive_named_entries)?;
        }
    }
    encoder.u64(payload.declarations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baggage::{
        BaggageInventory, BaggagePayload, MediaClass, NestedArchive, Resolution,
        ResolvedDeclaration,
    };
    use crate::package::{
        install_diagnostic_for_tests, InstallCategory, InstallDiagnosticKind, InstallDisposition,
        InstallFacts, InstallReport, PackageMember,
    };
    use crate::parse::baggage::BaggageDeclaration;
    use crate::report::{IdConflict, TranslationCounts, UnknownConstruct, UnknownKind};

    fn member(path: &str, role: &str) -> PackageMember {
        PackageMember {
            path: path.into(),
            role: role.into(),
            sha256: format!("sha-{path}-{role}"),
            size: 7,
        }
    }

    fn conflict(table: &str, id: &str) -> IdConflict {
        IdConflict {
            table: table.into(),
            id: id.into(),
            kept_sha256: format!("kept-{table}-{id}"),
            other_sha256: format!("other-{table}-{id}"),
            occurrence: 1,
        }
    }

    fn unknown(xpath: &str, name: &str) -> UnknownConstruct {
        UnknownConstruct {
            xpath: xpath.into(),
            kind: UnknownKind::Element,
            name: name.into(),
            occurrences: 2,
            sample: Some("sample".into()),
        }
    }

    fn diagnostic(detail: &str) -> InstallDiagnostic {
        install_diagnostic_for_tests(
            InstallDiagnosticKind::UnsupportedMasterSection,
            "a.knxprod".into(),
            "/xpath".into(),
            detail.into(),
            1,
        )
    }

    fn declaration(index_path: &str) -> ResolvedDeclaration {
        ResolvedDeclaration {
            index_path: index_path.into(),
            ordinal: 0,
            declaration: BaggageDeclaration {
                id: Some("id".into()),
                name: Some("name".into()),
                target_path: Some("target".into()),
                install_on_import: Some("false".into()),
                time_info: None,
                file_version: Some("1".into()),
            },
            resolution: Resolution::Resolved,
            member_path: Some("member".into()),
            detail: None,
        }
    }

    fn payload(member_path: &str) -> BaggagePayload {
        BaggagePayload {
            member_path: member_path.into(),
            sha256: format!("sha-{member_path}"),
            size: 3,
            media_class: MediaClass::Png,
            nested: NestedArchive::Read {
                entries: 1,
                declared_expanded_size: 2,
                encrypted_entries: 0,
                archive_named_entries: 0,
            },
            declarations: 1,
        }
    }

    fn base_report() -> InstallReport {
        InstallReport {
            sha256: "package-sha256".into(),
            scheme: 1,
            skipped: false,
            members: vec![member("M-1/knx_master.xml", "master")],
            unknown: 0,
            conflicts: vec![conflict("manufacturer", "M-1")],
            translations: TranslationCounts {
                program: 1,
                catalog: 2,
                hardware: 3,
                master: 4,
            },
            dropped_datapoint_types: 0,
            facts: Some(InstallFacts {
                counts: vec![crate::package::InstallCount {
                    category: InstallCategory::Product,
                    disposition: InstallDisposition::Stored,
                    count: 5,
                }],
                unknown_constructs: vec![unknown("/KNX/ManufacturerData", "Manufacturer")],
                unknown_occurrences: 2,
                diagnostics: vec![diagnostic("diag")],
            }),
            baggage: Some(BaggageInventory {
                declarations: vec![declaration("M-1/Baggages.xml")],
                payloads: vec![payload("M-1/image.png")],
            }),
            source_names: vec!["a.knxprod".into()],
        }
    }

    fn base_install() -> ProductFileInstall {
        ProductFileInstall {
            sha256: "file-sha256".into(),
            wrapper: Some(WrapperContents {
                other_members: vec![WrapperMember {
                    path: "README.md".into(),
                    size: 9,
                    sha256: "readme-sha".into(),
                    kind: WrapperMemberKind::File,
                }],
            }),
            packages: vec![InstalledPackage {
                path: Some("M-1/package.knxprod".into()),
                report: base_report(),
            }],
        }
    }

    fn identity(install: &ProductFileInstall) -> [u8; 32] {
        preview_identity(install).unwrap().sha256
    }

    #[test]
    fn identical_installs_encode_identically() {
        let first = preview_identity(&base_install()).unwrap();
        let second = preview_identity(&base_install()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.version, 1);
        assert_eq!(first.package_count, 1);
        assert_eq!(first.other_member_count, 1);
        assert_ne!(first.sha256, [0u8; 32]);
        assert_ne!(first.encoded_bytes, 0);
    }

    #[test]
    fn the_preview_identity_version_is_pinned() {
        const {
            assert!(PREVIEW_IDENTITY_VERSION == 1);
        }
    }

    #[test]
    fn the_file_sha256_is_discriminated() {
        let base = base_install();
        let mut changed = base_install();
        changed.sha256 = "other-file-sha256".into();
        assert_ne!(identity(&base), identity(&changed));
    }

    #[test]
    fn a_standalone_package_and_a_wrapper_encode_differently() {
        let base = base_install();
        let mut standalone = base_install();
        standalone.wrapper = None;
        assert_ne!(identity(&base), identity(&standalone));
    }

    #[test]
    fn an_empty_wrapper_is_distinct_from_no_wrapper() {
        let mut none = base_install();
        none.wrapper = None;
        let mut empty = base_install();
        empty.wrapper = Some(WrapperContents {
            other_members: Vec::new(),
        });
        assert_ne!(identity(&none), identity(&empty));
    }

    #[test]
    fn a_wrapper_member_change_is_discriminated() {
        let base = base_install();
        let mut changed = base_install();
        changed.wrapper.as_mut().unwrap().other_members[0].path = "LICENSE".into();
        assert_ne!(identity(&base), identity(&changed));
        let mut kind_changed = base_install();
        kind_changed.wrapper.as_mut().unwrap().other_members[0].kind =
            WrapperMemberKind::AppleDouble;
        assert_ne!(identity(&base), identity(&kind_changed));
    }

    #[test]
    fn a_package_without_a_path_is_distinct_from_an_empty_path() {
        let mut none = base_install();
        none.packages[0].path = None;
        let mut empty = base_install();
        empty.packages[0].path = Some(String::new());
        assert_ne!(identity(&none), identity(&empty));
    }

    #[test]
    fn the_report_sha256_and_scheme_are_discriminated() {
        let base = base_install();
        let mut sha = base_install();
        sha.packages[0].report.sha256 = "other-report-sha".into();
        assert_ne!(identity(&base), identity(&sha));
        let mut scheme = base_install();
        scheme.packages[0].report.scheme = 2;
        assert_ne!(identity(&base), identity(&scheme));
    }

    #[test]
    fn the_skipped_flag_is_discriminated() {
        let base = base_install();
        let mut skipped = base_install();
        skipped.packages[0].report.skipped = true;
        assert_ne!(identity(&base), identity(&skipped));
    }

    #[test]
    fn a_member_change_is_discriminated() {
        let base = base_install();
        let mut changed = base_install();
        changed.packages[0].report.members[0].path = "M-1/hardware.xml".into();
        assert_ne!(identity(&base), identity(&changed));
        let mut role = base_install();
        role.packages[0].report.members[0].role = "catalog".into();
        assert_ne!(identity(&base), identity(&role));
    }

    #[test]
    fn the_unknown_count_and_conflicts_are_discriminated() {
        let base = base_install();
        let mut unknown = base_install();
        unknown.packages[0].report.unknown = 1;
        assert_ne!(identity(&base), identity(&unknown));
        let mut conflict = base_install();
        conflict.packages[0].report.conflicts[0].other_sha256 = "different".into();
        assert_ne!(identity(&base), identity(&conflict));
    }

    #[test]
    fn each_translation_count_is_discriminated() {
        let base = base_install();
        for scope in ["program", "catalog", "hardware", "master"] {
            let mut changed = base_install();
            let translations = &mut changed.packages[0].report.translations;
            match scope {
                "program" => translations.program += 1,
                "catalog" => translations.catalog += 1,
                "hardware" => translations.hardware += 1,
                "master" => translations.master += 1,
                _ => unreachable!(),
            }
            assert_ne!(
                identity(&base),
                identity(&changed),
                "translation scope {scope} must discriminate"
            );
        }
    }

    #[test]
    fn the_dropped_datapoint_count_and_source_names_are_discriminated() {
        let base = base_install();
        let mut dropped = base_install();
        dropped.packages[0].report.dropped_datapoint_types = 1;
        assert_ne!(identity(&base), identity(&dropped));
        let mut name = base_install();
        name.packages[0].report.source_names = vec!["b.knxprod".into()];
        assert_ne!(identity(&base), identity(&name));
    }

    #[test]
    fn absent_facts_is_distinct_from_empty_facts() {
        let mut none = base_install();
        none.packages[0].report.facts = None;
        let mut empty = base_install();
        empty.packages[0].report.facts = Some(InstallFacts::default());
        assert_ne!(identity(&none), identity(&empty));
    }

    #[test]
    fn a_facts_count_and_unknown_construct_are_discriminated() {
        let base = base_install();
        let mut count = base_install();
        count.packages[0].report.facts.as_mut().unwrap().counts[0].count = 6;
        assert_ne!(identity(&base), identity(&count));
        let mut disposition = base_install();
        disposition.packages[0]
            .report
            .facts
            .as_mut()
            .unwrap()
            .counts[0]
            .disposition = InstallDisposition::Dropped;
        assert_ne!(identity(&base), identity(&disposition));
        let mut unknown = base_install();
        unknown.packages[0]
            .report
            .facts
            .as_mut()
            .unwrap()
            .unknown_constructs[0]
            .name = "Other".into();
        assert_ne!(identity(&base), identity(&unknown));
    }

    #[test]
    fn a_none_sample_is_distinct_from_a_some_empty_sample() {
        let mut none = base_install();
        none.packages[0]
            .report
            .facts
            .as_mut()
            .unwrap()
            .unknown_constructs[0]
            .sample = None;
        let mut empty = base_install();
        empty.packages[0]
            .report
            .facts
            .as_mut()
            .unwrap()
            .unknown_constructs[0]
            .sample = Some(String::new());
        assert_ne!(identity(&none), identity(&empty));
    }

    #[test]
    fn a_diagnostic_change_is_discriminated() {
        let base = base_install();
        let mut changed = base_install();
        changed.packages[0]
            .report
            .facts
            .as_mut()
            .unwrap()
            .diagnostics = vec![diagnostic("other")];
        assert_ne!(identity(&base), identity(&changed));
    }

    #[test]
    fn absent_baggage_is_distinct_from_empty_baggage() {
        let mut none = base_install();
        none.packages[0].report.baggage = None;
        let mut empty = base_install();
        empty.packages[0].report.baggage = Some(BaggageInventory {
            declarations: Vec::new(),
            payloads: Vec::new(),
        });
        assert_ne!(identity(&none), identity(&empty));
    }

    #[test]
    fn a_baggage_declaration_change_is_discriminated() {
        let base = base_install();
        let mut changed = base_install();
        changed.packages[0]
            .report
            .baggage
            .as_mut()
            .unwrap()
            .declarations[0]
            .resolution = Resolution::Missing;
        assert_ne!(identity(&base), identity(&changed));
    }

    #[test]
    fn a_baggage_declaration_none_detail_is_distinct_from_some_empty_detail() {
        let mut none = base_install();
        none.packages[0]
            .report
            .baggage
            .as_mut()
            .unwrap()
            .declarations[0]
            .detail = None;
        let mut empty = base_install();
        empty.packages[0]
            .report
            .baggage
            .as_mut()
            .unwrap()
            .declarations[0]
            .detail = Some(String::new());
        assert_ne!(identity(&none), identity(&empty));
    }

    #[test]
    fn a_baggage_payload_change_is_discriminated() {
        let base = base_install();
        let mut changed = base_install();
        changed.packages[0]
            .report
            .baggage
            .as_mut()
            .unwrap()
            .payloads[0]
            .media_class = MediaClass::Pdf;
        assert_ne!(identity(&base), identity(&changed));
        let mut nested = base_install();
        nested.packages[0].report.baggage.as_mut().unwrap().payloads[0].nested =
            NestedArchive::NotArchive;
        assert_ne!(identity(&base), identity(&nested));
    }

    #[test]
    fn a_package_count_and_member_count_are_reported_exactly() {
        let mut install = base_install();
        install.packages.push(InstalledPackage {
            path: None,
            report: base_report(),
        });
        let identity = preview_identity(&install).unwrap();
        assert_eq!(identity.package_count, 2);
        assert_eq!(identity.other_member_count, 1);
    }
}
