//! Resolves the display name a raw group address carries across all installations.
//!
//! A telegram carries only the raw 16-bit address. Installations are
//! separate address spaces (ADR-0038), so several of them may name the same
//! raw address. A monitor that keyed names by raw address and let the last
//! installation win showed one installation's name and hid the others. This
//! is the name counterpart of [`crate::resolve_project_group_address_dpts`],
//! which reports disagreeing DPTs as a conflict rather than picking one.

use std::collections::HashMap;

use crate::project::Project;

/// Separator between distinct names of one raw address.
pub const GROUP_ADDRESS_NAME_SEPARATOR: &str = " | ";

/// Maps every raw group address in `project` to its display name.
///
/// An address named once keeps that name exactly, including an empty one.
/// An address named by several entries lists each distinct name once, in
/// installation order and then entry order, joined by
/// [`GROUP_ADDRESS_NAME_SEPARATOR`]; empty names are left out of such a list
/// unless every entry is empty. The separator can itself occur inside a
/// name, so the joined text is for display, not for parsing.
pub fn resolve_project_group_address_names(project: &Project) -> HashMap<u16, String> {
    let mut collected: HashMap<u16, Vec<&str>> = HashMap::new();
    for installation in &project.installations {
        for entry in &installation.group_addresses {
            let names = collected.entry(entry.address.raw()).or_default();
            if !names.contains(&entry.name.as_str()) {
                names.push(&entry.name);
            }
        }
    }
    collected
        .into_iter()
        .map(|(raw, names)| (raw, join_names(&names)))
        .collect()
}

fn join_names(names: &[&str]) -> String {
    let non_empty: Vec<&str> = names.iter().copied().filter(|n| !n.is_empty()).collect();
    if non_empty.is_empty() {
        String::new()
    } else {
        non_empty.join(GROUP_ADDRESS_NAME_SEPARATOR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::GroupAddress;
    use crate::commissioning::CompletionStatus;
    use crate::group::GroupAddressEntry;
    use crate::ids::{GroupAddressId, InstallationId, SourceRef};
    use crate::installation::Installation;
    use crate::string_table::Language;
    use crate::topology::Topology;

    fn installation(id: u8, entries: &[(u16, &str)]) -> Installation {
        Installation {
            id: InstallationId(id),
            name: format!("I{id}"),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: entries
                .iter()
                .enumerate()
                .map(|(i, &(raw, name))| GroupAddressEntry {
                    id: GroupAddressId(u32::from(id) * 100 + i as u32),
                    source: SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: name.into(),
                    address: GroupAddress::from_raw(raw),
                    central: false,
                    unfiltered: false,
                    range: None,
                })
                .collect(),
            parameters: vec![],
        }
    }

    fn names(installations: Vec<Installation>) -> HashMap<u16, String> {
        let mut project = Project::new(Language("en".into()));
        project.installations = installations;
        resolve_project_group_address_names(&project)
    }

    #[test]
    fn a_single_name_is_kept_exactly_even_when_empty() {
        let map = names(vec![installation(0, &[(1, "Light"), (2, "")])]);
        assert_eq!(map[&1], "Light");
        assert_eq!(map[&2], "");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn distinct_names_are_listed_in_installation_order_once_each() {
        let map = names(vec![
            installation(0, &[(1, "Home light")]),
            installation(1, &[(1, "Garage light")]),
            installation(2, &[(1, "Home light")]),
        ]);
        assert_eq!(map[&1], "Home light | Garage light");
    }

    #[test]
    fn empty_names_do_not_pad_a_list_but_an_all_empty_address_stays_empty() {
        let map = names(vec![
            installation(0, &[(1, ""), (2, "")]),
            installation(1, &[(1, "Garage light"), (2, "")]),
        ]);
        assert_eq!(map[&1], "Garage light");
        assert_eq!(map[&2], "");
    }

    #[test]
    fn a_repeated_address_inside_one_installation_is_listed_too() {
        let map = names(vec![installation(0, &[(1, "A"), (1, "B")])]);
        assert_eq!(map[&1], "A | B");
    }
}
