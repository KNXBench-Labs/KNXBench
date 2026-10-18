//! Vendor wrapper ZIPs (ADR-0140).
//!
//! Manufacturers often ship a plain ZIP that holds one or more `.knxprod`
//! packages next to manuals and other files, but no `knx_master.xml` of its
//! own. Such a wrapper passes the same structural, identity, path and size
//! checks as a package; then every inner package is installed byte-exact,
//! as if it had been given on its own, all in one transaction. Every other
//! member is named in the result with its size and digest (macOS AppleDouble
//! metadata named like a package included); nothing is
//! skipped silently, and nothing besides the inner packages is stored.
//! A refused inner package refuses the whole wrapper. Wrappers do not nest.

use rusqlite::Connection;
use serde::Serialize;

use crate::package::{
    admit_package_bytes, install_admitted_package, is_inner_package_path, open_checked_archive,
    read_checked_member, CheckedArchive,
};
use crate::{sha256_hex, InstallReport, PackageError, PackageLimits, ProductDbError};

/// Joins a wrapper's source name and an inner member path into the source
/// name the inner package is recorded under (`vendor.zip!/dir/a.knxprod`).
pub const WRAPPER_SOURCE_SEPARATOR: &str = "!/";

/// RFC 1740 (AppleSingle/AppleDouble) §2.2: the AppleDouble header magic.
/// macOS archivers add such a sidecar (`__MACOSX/._name`) for every file, so a
/// vendor ZIP built on a Mac carries one named like each inner package.
const APPLE_DOUBLE_MAGIC: [u8; 4] = [0x00, 0x05, 0x16, 0x07];

/// What a reported wrapper member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WrapperMemberKind {
    /// Any file that is not a product package (manual, legacy database, ...).
    File,
    /// macOS file metadata (AppleDouble, recognised by its magic), even when
    /// it is named like a `.knxprod`.
    AppleDouble,
}

/// A wrapper member that is not a product package: reported, not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapperMember {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub kind: WrapperMemberKind,
}

/// What a wrapper carried besides its product packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapperContents {
    pub other_members: Vec<WrapperMember>,
}

/// One installed package: the given file itself (`path` is `None`) or one
/// inner package of a wrapper (`path` is its member path).
#[derive(Debug)]
pub struct InstalledPackage {
    pub path: Option<String>,
    pub report: InstallReport,
}

/// The outcome of [`install_product_file`].
#[derive(Debug)]
pub struct ProductFileInstall {
    /// SHA-256 of the file as given (the package or the wrapper).
    pub sha256: String,
    /// `None` for a product package.
    pub wrapper: Option<WrapperContents>,
    /// In archive order; exactly one for a product package.
    pub packages: Vec<InstalledPackage>,
}

/// Installs a product package or a vendor wrapper of product packages.
/// Everything happens in one transaction: either every inner package is
/// installed or none is.
pub fn install_product_file(
    conn: &Connection,
    source_name: &str,
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<ProductFileInstall, PackageError> {
    admit_package_bytes(source_name, bytes)?;
    let tx = conn.unchecked_transaction().map_err(ProductDbError::from)?;
    let installed = install_admitted_product_file(&tx, source_name, bytes, limits)?;
    tx.commit().map_err(ProductDbError::from)?;
    Ok(installed)
}

/// [`install_product_file`] inside the caller's transaction, after
/// admission. Commits nothing.
pub(crate) fn install_admitted_product_file(
    tx: &Connection,
    source_name: &str,
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<ProductFileInstall, PackageError> {
    let sha256 = sha256_hex(bytes);
    match install_admitted_package(tx, source_name, bytes, limits) {
        Ok(report) => Ok(ProductFileInstall {
            sha256,
            wrapper: None,
            packages: vec![InstalledPackage { path: None, report }],
        }),
        // Refused at the master check, before the first write.
        Err(PackageError::WrapperArchive { .. }) => {
            install_wrapper(tx, source_name, bytes, limits, sha256)
        }
        Err(error) => Err(error),
    }
}

fn install_wrapper(
    tx: &Connection,
    source_name: &str,
    bytes: &[u8],
    limits: PackageLimits,
    sha256: String,
) -> Result<ProductFileInstall, PackageError> {
    let CheckedArchive { mut archive, .. } = open_checked_archive(bytes, limits)?;
    // The inner packages share one expansion budget: a wrapper never costs
    // more than one package of the same profile may.
    let mut remaining = limits.max_expanded_size;
    let mut packages = Vec::new();
    let mut other_members = Vec::new();
    for index in 0..archive.len() {
        let (path, data) = read_checked_member(&mut archive, index, limits.max_member_size)?;
        let Some(data) = data else {
            continue;
        };
        let size = u64::try_from(data.len())
            .map_err(|_| PackageError::SizeLimit { path: path.clone() })?;
        let apple_double = data.starts_with(&APPLE_DOUBLE_MAGIC);
        if !is_inner_package_path(&path) || apple_double {
            other_members.push(WrapperMember {
                sha256: sha256_hex(&data),
                path,
                size,
                kind: if apple_double {
                    WrapperMemberKind::AppleDouble
                } else {
                    WrapperMemberKind::File
                },
            });
            continue;
        }
        let inner_name = format!("{source_name}{WRAPPER_SOURCE_SEPARATOR}{path}");
        let inner_limits = PackageLimits {
            max_member_size: limits.max_member_size,
            max_expanded_size: remaining,
        };
        let refused = |error: PackageError| PackageError::WrapperMember {
            path: path.clone(),
            error: Box::new(error),
        };
        admit_package_bytes(&inner_name, &data).map_err(refused)?;
        let report =
            install_admitted_package(tx, &inner_name, &data, inner_limits).map_err(refused)?;
        let expanded = report
            .members
            .iter()
            .try_fold(0_u64, |sum, member| sum.checked_add(member.size))
            .and_then(|expanded| remaining.checked_sub(expanded))
            .ok_or_else(|| refused(PackageError::SizeLimit { path: path.clone() }))?;
        remaining = expanded;
        packages.push(InstalledPackage {
            path: Some(path),
            report,
        });
    }
    // Every `.knxprod` was macOS metadata: the ZIP holds no package at all.
    if packages.is_empty() {
        return Err(PackageError::MissingMaster);
    }
    Ok(ProductFileInstall {
        sha256,
        wrapper: Some(WrapperContents { other_members }),
        packages,
    })
}
