//! Declared mask version to KNX system family, from one cited table.
//!
//! WP-20: the Devices list names each device's system family from the mask
//! version its application program declares in the product database. This
//! module is the single place that mapping lives; the UI only translates the
//! closed [`SystemFamily`] vocabulary into words.
//!
//! A mask version *read from a device* (Device Descriptor Type 0) is a
//! separate, session-only fact and is never an input here.

use crate::commissioning::load_state::MaskVersion;

/// The system family or other named S-Mode profile a mask version belongs to.
///
/// Families come from the sources cited on [`MASK_FAMILIES`]; a mask the
/// table does not list has no family and is reported as unknown, never
/// guessed from neighbouring values or bit fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemFamily {
    System1,
    System2,
    System7,
    SystemB,
    System300,
    Coupler,
    KnxnetIpRouter,
    UsbInterface,
    IrDecoder,
    RfBidirectional,
    RfUnidirectional,
}

/// What a device's declared mask version says about its system family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredSystem {
    /// The application program declares no mask version.
    NoMaskVersion,
    /// The declared text is not an `MV-xxxx` reference; kept verbatim.
    InvalidMask(String),
    /// A well-formed mask version the table does not list.
    UnknownMask(MaskVersion),
    /// A mask version the table maps to a family.
    Family {
        mask: MaskVersion,
        family: SystemFamily,
    },
}

/// The cited mask-version → family table.
///
/// Sources (KNX Standard v3.0.0, original PDFs, printed pages):
///
/// - **RES** — `03_05_01 Resources v01.10.01`, §4.1.2.2 Table 7 “KNX
///   Association approved mask versions for devices”, pp25–26 (Device
///   Profile column).
/// - **PROF** — `06 Profiles v02.01.01`, §1.2 Table 1 “Overview S-Mode
///   Configuration Profiles (informative)”, p13 (Profile Class column).
/// - **M2705** — `06_01_33 mask 2705h` (footer v01.01.01), §1, p6:
///   System 7 on KNX RF.
/// - **M27B0** — `06_01_35 mask 27B0h v01.01.02`, §1, p6: System B profile
///   class on KNX RF.
/// - **M2920** — `06_02_42 mask 2920h v01.01.01` “S-Mode Profiles for
///   Couplers”, §1.1, p4: base profile for a TP1/RF media coupler.
///
/// Rows backed by both RES and PROF: `0010h`–`0013h`, `1012h`, `1013h`,
/// `3012h`, `4012h`, `0020h`, `0021h`, `0025h`, `0300h`, `0700h`, `0701h`,
/// `0705h`, `5705h`, `07B0h`, `17B0h`, `0910h`–`0912h`, `1900h` and `091Ah`.
/// Every other row has a single source, named in its comment below.
///
/// Deliberately absent: `0AFDh` and `0AFEh`, whose RES device profile is
/// “none (see V06 Profiles)” and which PROF Table 1 does not list.
pub const MASK_FAMILIES: &[(u16, SystemFamily)] = &[
    // RES “BCU 1, System 1” (TP1) / “BCU 1” (PL110, TP0, PL132); PROF System 1.
    (0x0010, SystemFamily::System1),
    (0x0011, SystemFamily::System1),
    (0x0012, SystemFamily::System1),
    (0x0013, SystemFamily::System1),
    (0x1012, SystemFamily::System1),
    (0x1013, SystemFamily::System1),
    (0x3012, SystemFamily::System1),
    (0x4012, SystemFamily::System1),
    // RES “BCU 2, System 2”; PROF System 2.
    (0x0020, SystemFamily::System2),
    (0x0021, SystemFamily::System2),
    (0x0025, SystemFamily::System2),
    // RES “System 300”; PROF System 300.
    (0x0300, SystemFamily::System300),
    // Single source PROF: System 300 (RES does not list 2300h).
    (0x2300, SystemFamily::System300),
    // RES “BIM M112” (070nh) and “System 7” (5705h); PROF System 7 lists all four.
    (0x0700, SystemFamily::System7),
    (0x0701, SystemFamily::System7),
    (0x0705, SystemFamily::System7),
    (0x5705, SystemFamily::System7),
    // Single source M2705: System 7 on KNX RF (not in RES or PROF).
    (0x2705, SystemFamily::System7),
    // RES “System B” (07B0h, 17B0h); PROF System B (07B0h, 17B0h).
    (0x07B0, SystemFamily::SystemB),
    (0x17B0, SystemFamily::SystemB),
    // Single source RES “System B” on KNX IP (PROF Table 1 does not list it).
    (0x57B0, SystemFamily::SystemB),
    // Single source M27B0: System B profile class on KNX RF.
    (0x27B0, SystemFamily::SystemB),
    // RES “Coupler 1.0/1.1/1.2”, “Media Coupler PL-TP”; PROF Coupler.
    (0x0910, SystemFamily::Coupler),
    (0x0911, SystemFamily::Coupler),
    (0x0912, SystemFamily::Coupler),
    (0x1900, SystemFamily::Coupler),
    // Single source M2920: TP1/RF media coupler base profile.
    (0x2920, SystemFamily::Coupler),
    // RES “KNXnet/IP Router”; PROF KNXnet/IP Router.
    (0x091A, SystemFamily::KnxnetIpRouter),
    // Single source RES: “TP1/PL110 USB interface v1/v2”, “RF USB interface v2”.
    (0x0310, SystemFamily::UsbInterface),
    (0x0311, SystemFamily::UsbInterface),
    (0x1310, SystemFamily::UsbInterface),
    (0x1311, SystemFamily::UsbInterface),
    (0x2311, SystemFamily::UsbInterface),
    // Single source RES: “IR-Decoder”.
    (0x0810, SystemFamily::IrDecoder),
    (0x0811, SystemFamily::IrDecoder),
    // Single source RES: RF “Bi-directional devices” / “Uni-directional devices”.
    (0x2010, SystemFamily::RfBidirectional),
    (0x2110, SystemFamily::RfUnidirectional),
];

/// The family for `mask`, or `None` when the table does not list it.
pub fn system_family(mask: MaskVersion) -> Option<SystemFamily> {
    MASK_FAMILIES
        .iter()
        .find(|(value, _)| *value == mask.0)
        .map(|(_, family)| *family)
}

/// Parses a product-database mask reference such as `MV-0701`: the exact
/// `MV-` prefix and four hexadecimal digits (either case), nothing else.
pub fn parse_mask_version_ref(text: &str) -> Option<MaskVersion> {
    let digits = text.strip_prefix("MV-")?;
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u16::from_str_radix(digits, 16).ok().map(MaskVersion)
}

/// Classifies an application program's declared mask version text.
pub fn declared_system(mask_version: Option<&str>) -> DeclaredSystem {
    let Some(text) = mask_version else {
        return DeclaredSystem::NoMaskVersion;
    };
    let Some(mask) = parse_mask_version_ref(text) else {
        return DeclaredSystem::InvalidMask(text.to_string());
    };
    match system_family(mask) {
        Some(family) => DeclaredSystem::Family { mask, family },
        None => DeclaredSystem::UnknownMask(mask),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row the cited sources assign, written out independently of
    /// `MASK_FAMILIES` so a dropped, added or moved row fails here.
    const EXPECTED: &[(u16, SystemFamily)] = &[
        (0x0010, SystemFamily::System1),
        (0x0011, SystemFamily::System1),
        (0x0012, SystemFamily::System1),
        (0x0013, SystemFamily::System1),
        (0x1012, SystemFamily::System1),
        (0x1013, SystemFamily::System1),
        (0x3012, SystemFamily::System1),
        (0x4012, SystemFamily::System1),
        (0x0020, SystemFamily::System2),
        (0x0021, SystemFamily::System2),
        (0x0025, SystemFamily::System2),
        (0x0300, SystemFamily::System300),
        (0x2300, SystemFamily::System300),
        (0x0700, SystemFamily::System7),
        (0x0701, SystemFamily::System7),
        (0x0705, SystemFamily::System7),
        (0x5705, SystemFamily::System7),
        (0x2705, SystemFamily::System7),
        (0x07B0, SystemFamily::SystemB),
        (0x17B0, SystemFamily::SystemB),
        (0x57B0, SystemFamily::SystemB),
        (0x27B0, SystemFamily::SystemB),
        (0x0910, SystemFamily::Coupler),
        (0x0911, SystemFamily::Coupler),
        (0x0912, SystemFamily::Coupler),
        (0x1900, SystemFamily::Coupler),
        (0x2920, SystemFamily::Coupler),
        (0x091A, SystemFamily::KnxnetIpRouter),
        (0x0310, SystemFamily::UsbInterface),
        (0x0311, SystemFamily::UsbInterface),
        (0x1310, SystemFamily::UsbInterface),
        (0x1311, SystemFamily::UsbInterface),
        (0x2311, SystemFamily::UsbInterface),
        (0x0810, SystemFamily::IrDecoder),
        (0x0811, SystemFamily::IrDecoder),
        (0x2010, SystemFamily::RfBidirectional),
        (0x2110, SystemFamily::RfUnidirectional),
    ];

    #[test]
    fn every_cited_mask_maps_to_its_family() {
        for &(mask, family) in EXPECTED {
            assert_eq!(
                system_family(MaskVersion(mask)),
                Some(family),
                "mask {mask:04X}h"
            );
        }
    }

    #[test]
    fn the_table_holds_exactly_the_cited_rows_once() {
        let mut table = MASK_FAMILIES.to_vec();
        let mut expected = EXPECTED.to_vec();
        table.sort_by_key(|row| row.0);
        expected.sort_by_key(|row| row.0);
        assert_eq!(table, expected);
        let mut masks: Vec<u16> = MASK_FAMILIES.iter().map(|row| row.0).collect();
        masks.sort_unstable();
        masks.dedup();
        assert_eq!(masks.len(), MASK_FAMILIES.len(), "no mask listed twice");
    }

    #[test]
    fn owner_families_cover_system_1_2_7_and_b() {
        for (mask, family) in [
            (0x0012, SystemFamily::System1),
            (0x0025, SystemFamily::System2),
            (0x0701, SystemFamily::System7),
            (0x07B0, SystemFamily::SystemB),
        ] {
            assert_eq!(
                declared_system(Some(&format!("MV-{mask:04X}"))),
                DeclaredSystem::Family {
                    mask: MaskVersion(mask),
                    family
                }
            );
        }
    }

    #[test]
    fn masks_without_an_assigned_profile_stay_unknown() {
        // Resources Table 7 assigns 0AFDh/0AFEh "none"; the others are
        // neighbours of mapped rows that no cited source lists.
        for mask in [0x0AFD, 0x0AFE, 0x0702, 0x0014, 0x07B1, 0x0000, 0xFFFF] {
            assert_eq!(system_family(MaskVersion(mask)), None, "{mask:04X}h");
            assert_eq!(
                declared_system(Some(&format!("MV-{mask:04X}"))),
                DeclaredSystem::UnknownMask(MaskVersion(mask))
            );
        }
    }

    #[test]
    fn a_missing_declaration_is_not_an_unknown_mask() {
        assert_eq!(declared_system(None), DeclaredSystem::NoMaskVersion);
    }

    #[test]
    fn malformed_declarations_are_kept_verbatim() {
        for text in [
            "", "MV-07", "MV-07011", "0701", "mv-0701", "MV-+701", "MV-07G1", " MV-0701",
        ] {
            assert_eq!(parse_mask_version_ref(text), None, "{text:?}");
            assert_eq!(
                declared_system(Some(text)),
                DeclaredSystem::InvalidMask(text.to_string())
            );
        }
    }

    #[test]
    fn mask_references_parse_both_hex_cases() {
        assert_eq!(parse_mask_version_ref("MV-07B0"), Some(MaskVersion(0x07B0)));
        assert_eq!(parse_mask_version_ref("MV-07b0"), Some(MaskVersion(0x07B0)));
    }
}
