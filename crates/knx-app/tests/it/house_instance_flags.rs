//! The house's instance-level flags reach the group object table (KNOWN_LIMITATIONS §145).
//!
//! On 2026-09-29 every device of the maintainer's house was read back,
//! read-only (RESEARCH §19.13). Each active, linked object whose
//! `ComObjectInstanceRef` states a flag held that flag on the device, and
//! the download image used to drop it: 1.1.20 object 0 carries
//! `WriteFlag="Enabled" UpdateFlag="Enabled"` over a product that has both
//! off, and a KNXBench download would have stopped it accepting writes.
//!
//! The expected octets below are the config octets the devices hold, read
//! from their group object tables (`OriginalData/DeviceBackups/
//! house-readback-2026-09-29/`, private). Only objects the project links are
//! checked: for an active object without a link ETS clears the
//! communication bit and KNXBench does not (RESEARCH §19.13, cause 2), which
//! is a separate, behaviour-neutral difference.
//!
//! `#[ignore]`d: it needs the gitignored `OriginalData/` corpus.

use knx_core::commissioning::group_object_table::ObjectFlags;
use knx_productdb::image::build_download_image;
use knx_productdb::image_request::image_request_for_device;

/// `(device, object number, config octet on the device)`.
const READ_BACK: [(&str, u8, u8); 12] = [
    ("1.1.5", 0, 0x5F),
    ("1.1.20", 0, 0xDF),
    ("1.1.20", 5, 0xDF),
    ("1.1.20", 10, 0xDF),
    ("1.1.20", 15, 0x5F),
    ("1.1.21", 0, 0xDF),
    ("1.1.21", 5, 0xDF),
    ("1.1.21", 10, 0xDF),
    ("1.1.21", 15, 0x5F),
    ("1.1.32", 0, 0xDF),
    ("1.1.32", 5, 0xDF),
    ("1.1.32", 10, 0xDF),
];

/// The config octet of `flags`, as Resources §4.18.3/§4.18.4 lay it out
/// (segment selector 0, which every object checked here has in its base
/// image and on the device).
fn config_octet(flags: ObjectFlags) -> u8 {
    let bit = |on: bool, mask: u8| if on { mask } else { 0 };
    bit(flags.update, 0x80)
        | bit(flags.transmit, 0x40)
        | bit(flags.write, 0x10)
        | bit(flags.read, 0x08)
        | bit(flags.communication, 0x04)
        | flags.priority.bits()
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_houses_instance_flags_are_in_the_image_as_on_the_devices() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only)"
    );
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let project = knx_app::import_ets_project_with(
        &knx_testsupport::reference_ets6_path(),
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap()
    .project;

    for (address, number, on_device) in READ_BACK {
        let device = project
            .devices
            .iter()
            .find(|d| d.address == Some(address.parse().unwrap()))
            .unwrap_or_else(|| panic!("{address} not in the project"));
        let request = image_request_for_device(&products, &project, device.id)
            .unwrap_or_else(|e| panic!("{address}: {e}"));
        let image =
            build_download_image(&products, &request).unwrap_or_else(|e| panic!("{address}: {e}"));
        let object = image
            .objects
            .iter()
            .find(|o| o.number == number)
            .unwrap_or_else(|| panic!("{address} object {number} not active"));
        assert_eq!(
            config_octet(object.flags),
            on_device,
            "{address} object {number}: image {:02X}h, device {on_device:02X}h",
            config_octet(object.flags)
        );
    }
}
