//! Selects one project device by its individual address or by its internal id.
//!
//! `--device <area.line.device>` names a device the way every surface shows
//! it; `--device-id <id>` is the internal store id. Exactly one is required.
//! An address several devices share is refused with their ids, never guessed,
//! and a device without an assigned address is never selected by address.
use knx_core::{DeviceId, IndividualAddress, Project};

/// The device a command was told to work on, before the project is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSelector {
    /// `--device-id <id>`: the internal store id, taken as given.
    Id(DeviceId),
    /// `--device <area.line.device>`: resolved against the project's devices.
    Address(IndividualAddress),
}

impl DeviceSelector {
    /// Parses the values of `--device-id` and `--device`; exactly one is set.
    pub fn parse(device_id: Option<&str>, device: Option<&str>) -> Result<Self, String> {
        match (device_id, device) {
            (Some(_), Some(_)) => {
                Err("give either --device <area.line.device> or --device-id <id>, not both".into())
            }
            (None, None) => Err("--device <area.line.device> or --device-id <id> required".into()),
            (Some(id), None) => id
                .parse::<u32>()
                .map(|id| Self::Id(DeviceId(id)))
                .map_err(|_| "invalid device ID".into()),
            (None, Some(text)) => text.trim().parse().map(Self::Address).map_err(|e| {
                format!("--device must be area.line.device, e.g. 1.1.5 (got {text:?}): {e}")
            }),
        }
    }

    /// The selected device's id. An id is passed through unchanged (the
    /// command's own lookup reports a missing one); an address must belong
    /// to exactly one project device.
    pub fn resolve(self, project: &Project) -> Result<DeviceId, String> {
        let address = match self {
            Self::Id(id) => return Ok(id),
            Self::Address(address) => address,
        };
        let mut matches: Vec<_> = project
            .devices
            .iter()
            .filter(|device| device.address == Some(address))
            .collect();
        matches.sort_by_key(|device| device.id);
        match matches.as_slice() {
            [] => Err(format!(
                "no project device has individual address {address}; nothing selected"
            )),
            [one] => Ok(one.id),
            several => Err(format!(
                "{} project devices share individual address {address}; choose one with --device-id <id>: {}",
                several.len(),
                several
                    .iter()
                    .map(|device| format!("{} {:?}", device.id, device.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{DeviceInstance, Language, SourceRef};

    fn device(id: u32, name: &str, address: Option<&str>) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: SourceRef {
                path: String::new(),
                ets_id: String::new(),
            },
            name: name.into(),
            description: None,
            address: address.map(|a| a.parse().unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn project(devices: Vec<DeviceInstance>) -> Project {
        let mut p = Project::new(Language("en".into()));
        for d in devices {
            p.devices.insert(d);
        }
        p
    }

    fn address(text: &str) -> DeviceSelector {
        DeviceSelector::Address(text.parse().unwrap())
    }

    #[test]
    fn an_address_or_an_id_parses_into_its_selector() {
        assert_eq!(
            DeviceSelector::parse(None, Some("1.1.67")),
            Ok(address("1.1.67"))
        );
        assert_eq!(
            DeviceSelector::parse(Some("12"), None),
            Ok(DeviceSelector::Id(DeviceId(12)))
        );
    }

    #[test]
    fn both_flags_are_refused() {
        let error = DeviceSelector::parse(Some("1"), Some("1.1.67")).unwrap_err();
        assert!(error.contains("not both"), "{error}");
        assert!(
            error.contains("--device ") && error.contains("--device-id"),
            "{error}"
        );
    }

    #[test]
    fn neither_flag_is_refused_naming_both() {
        let error = DeviceSelector::parse(None, None).unwrap_err();
        assert!(error.contains("--device <area.line.device>"), "{error}");
        assert!(error.contains("--device-id <id>"), "{error}");
    }

    #[test]
    fn a_malformed_address_is_a_parse_error_naming_the_input() {
        for bad in [
            "1.1", "16.0.1", "1.16.1", "1.1.256", "1.1.x", "", "1.1.1.1", "-1.1.1",
        ] {
            let error = DeviceSelector::parse(None, Some(bad)).unwrap_err();
            assert!(error.contains("area.line.device"), "{bad}: {error}");
            assert!(error.contains(&format!("{bad:?}")), "{bad}: {error}");
        }
    }

    #[test]
    fn a_malformed_id_is_refused() {
        assert!(DeviceSelector::parse(Some("1.1.67"), None).is_err());
    }

    #[test]
    fn the_one_device_at_an_address_is_selected() {
        let p = project(vec![
            device(1, "Dimmer", Some("1.1.5")),
            device(2, "Actuator", Some("1.1.67")),
        ]);
        assert_eq!(address("1.1.67").resolve(&p), Ok(DeviceId(2)));
    }

    #[test]
    fn an_unknown_address_is_refused_naming_it() {
        let p = project(vec![device(1, "Dimmer", Some("1.1.5"))]);
        let error = address("1.1.9").resolve(&p).unwrap_err();
        assert!(error.contains("1.1.9"), "{error}");
        assert!(error.contains("no project device"), "{error}");
    }

    #[test]
    fn devices_without_an_address_never_match() {
        let p = project(vec![device(1, "Unaddressed", None)]);
        let error = address("0.0.0").resolve(&p).unwrap_err();
        assert!(error.contains("0.0.0"), "{error}");
    }

    #[test]
    fn a_shared_address_is_refused_listing_every_candidate_by_id() {
        let p = project(vec![
            device(7, "Second actuator", Some("1.1.67")),
            device(3, "First actuator", Some("1.1.67")),
            device(5, "Elsewhere", Some("1.1.68")),
        ]);
        let error = address("1.1.67").resolve(&p).unwrap_err();
        assert!(error.contains("1.1.67"), "{error}");
        assert!(error.contains("--device-id"), "{error}");
        let first = error.find("3 \"First actuator\"").expect(&error);
        let second = error.find("7 \"Second actuator\"").expect(&error);
        assert!(first < second, "candidates in id order: {error}");
        assert!(!error.contains("Elsewhere"), "{error}");
    }

    #[test]
    fn an_id_is_passed_through_unchanged() {
        let p = project(vec![]);
        assert_eq!(DeviceSelector::Id(DeviceId(4)).resolve(&p), Ok(DeviceId(4)));
    }
}
