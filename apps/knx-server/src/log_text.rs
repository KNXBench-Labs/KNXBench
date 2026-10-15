//! Plain-language session-log messages for commands a person reads (#13).
//!
//! The log's `message` used to be the command's variant name and its
//! `detail` the full `Debug` dump, so a link read
//! `LinkComObject … LinkComObject { com_object: ComObjectInstanceId(153) … }`.
//! A described command gets a sentence with the names a user knows; the
//! dump stays in `detail` byte for byte, and every other command keeps the
//! old message. Described from the project *before* the command runs, so an
//! unlink still finds what it unlinks. English like every server message.
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{
    string_table::Text, ComObjectInstanceId, Command, DeviceInstance, Direction, GroupAddressId,
    Override, Project,
};

/// A sentence for `cmd`, or `None` to keep the command name.
pub(crate) fn describe(project: &Project, cmd: &Command) -> Option<String> {
    match cmd {
        Command::LinkComObject {
            com_object,
            ga,
            direction,
        } => link_sentence(project, true, *com_object, *ga, direction_word(direction)),
        Command::UnlinkComObject {
            com_object,
            ga,
            direction,
        } => link_sentence(project, false, *com_object, *ga, direction_word(direction)),
        Command::SetDeviceProgramming { device, change } => {
            let device = project.devices.get(*device)?;
            programming_sentence(device, change)
        }
        // "Both" from the web is one batch of a send and a receive link.
        Command::Batch(commands) => match commands.as_slice() {
            [Command::LinkComObject {
                com_object: a,
                ga: x,
                direction: Direction::Send,
            }, Command::LinkComObject {
                com_object: b,
                ga: y,
                direction: Direction::Receive,
            }] if a == b && x == y => link_sentence(project, true, *a, *x, "send and receive"),
            [Command::UnlinkComObject {
                com_object: a,
                ga: x,
                direction: Direction::Send,
            }, Command::UnlinkComObject {
                com_object: b,
                ga: y,
                direction: Direction::Receive,
            }] if a == b && x == y => link_sentence(project, false, *a, *x, "send and receive"),
            _ => None,
        },
        _ => None,
    }
}

fn direction_word(direction: &Direction) -> &'static str {
    match direction {
        Direction::Send => "send",
        Direction::Receive => "receive",
    }
}

fn link_sentence(
    project: &Project,
    link: bool,
    com_object: ComObjectInstanceId,
    ga: GroupAddressId,
    direction: &str,
) -> Option<String> {
    let object = project.devices.com_object(com_object)?;
    let device = project.devices.get(object.device)?;
    let address = project
        .installations
        .iter()
        .flat_map(|installation| &installation.group_addresses)
        .find(|entry| entry.id == ga)?;
    let object_text = match &object.text {
        Override::Value(text) => text_of(project, &text.value),
        _ => None,
    };
    let object_label = match object_text {
        Some(text) if !text.is_empty() => format!("object {} “{}”", object.number, text),
        _ => format!("object {}", object.number),
    };
    let device_label = device_label(device);
    let ga_label = format!(
        "{} “{}”",
        address.address.format(project.info.group_address_style),
        address.name
    );
    Some(if link {
        format!("Linked {object_label} of {device_label} to {ga_label} ({direction})")
    } else {
        format!("Unlinked {object_label} of {device_label} from {ga_label} ({direction})")
    })
}

/// A device as the log names it: “Name” (1.1.5).
pub(crate) fn device_label(device: &DeviceInstance) -> String {
    match &device.address {
        Some(individual) => format!("“{}” ({individual})", device.name),
        None => format!("“{}”", device.name),
    }
}

/// The parts in reading order: "address, parameters and group addresses".
pub(crate) fn parts_text(parts: ProgrammingParts) -> String {
    let names: Vec<&str> = [
        (parts.address, "address"),
        (parts.application, "application"),
        (parts.parameters, "parameters"),
        (parts.group_addresses, "group addresses"),
    ]
    .into_iter()
    .filter_map(|(member, name)| member.then_some(name))
    .collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// "Marked as programmed" for all four parts, otherwise the parts named.
fn programming_sentence(device: &DeviceInstance, change: &ProgrammingChange) -> Option<String> {
    let label = device_label(device);
    let set = |value: bool| ProgrammingParts {
        address: change.address == Some(value),
        application: change.application == Some(value),
        parameters: change.parameters == Some(value),
        group_addresses: change.group_addresses == Some(value),
    };
    let sentences: Vec<String> = [(true, "programmed"), (false, "not programmed")]
        .into_iter()
        .filter_map(|(value, word)| {
            let parts = set(value);
            if parts == ProgrammingParts::ALL {
                Some(format!("{label} marked as {word}"))
            } else if parts.is_empty() {
                None
            } else {
                Some(format!("{label}: {} marked as {word}", parts_text(parts)))
            }
        })
        .collect();
    (!sentences.is_empty()).then(|| sentences.join("; "))
}

fn text_of(project: &Project, text: &Text) -> Option<String> {
    project
        .strings
        .text(text, project.strings.default_language())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        string_table::Language, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
        GroupAddress, GroupAddressEntry, GroupAddressStyle, IndividualAddress, Installation,
        InstallationId, Layer, Resolved, ResolvedFlags, SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    /// A fictional actuator with object 3 "Switch" and one address 1/2/3.
    fn project(text: Override<Text>, address: Option<IndividualAddress>) -> Project {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![DeviceId(1)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![GroupAddressEntry {
                id: GroupAddressId(7),
                source: source(),
                name: "Kitchen light".into(),
                address: GroupAddress::from_raw((1 << 11) | (2 << 8) | 3),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            }],
            parameters: vec![],
        });
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "Kitchen actuator".into(),
            description: None,
            address,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(153)],
            binary_data: vec![],
        });
        project
            .devices
            .insert_com_object(knx_core::ComObjectInstance {
                id: ComObjectInstanceId(153),
                source: source(),
                device: DeviceId(1),
                number: 3,
                text,
                description: Override::Absent,
                dpt: Override::Absent,
                flags: ResolvedFlags::none(),
                size: None,
                is_active: true,
                links: vec![],
                module_instance: None,
            });
        project
    }

    fn switch() -> Override<Text> {
        Override::Value(Resolved {
            value: Text::Literal("Switch".into()),
            layer: Layer::Instance,
        })
    }

    fn link(direction: Direction) -> Command {
        Command::LinkComObject {
            com_object: ComObjectInstanceId(153),
            ga: GroupAddressId(7),
            direction,
        }
    }

    fn unlink(direction: Direction) -> Command {
        Command::UnlinkComObject {
            com_object: ComObjectInstanceId(153),
            ga: GroupAddressId(7),
            direction,
        }
    }

    #[test]
    fn a_link_reads_like_a_sentence_with_names() {
        let project = project(switch(), Some("1.1.5".parse().unwrap()));
        assert_eq!(
            describe(&project, &link(Direction::Send)).unwrap(),
            "Linked object 3 “Switch” of “Kitchen actuator” (1.1.5) to 1/2/3 “Kitchen light” (send)"
        );
        assert_eq!(
            describe(&project, &unlink(Direction::Receive)).unwrap(),
            "Unlinked object 3 “Switch” of “Kitchen actuator” (1.1.5) from 1/2/3 “Kitchen light” (receive)"
        );
    }

    #[test]
    fn both_directions_are_one_sentence_and_follow_the_address_style() {
        let mut project = project(Override::Absent, None);
        project.info.group_address_style = GroupAddressStyle::TwoLevel;
        let both = Command::Batch(vec![link(Direction::Send), link(Direction::Receive)]);
        assert_eq!(
            describe(&project, &both).unwrap(),
            "Linked object 3 of “Kitchen actuator” to 1/515 “Kitchen light” (send and receive)"
        );
        let both = Command::Batch(vec![unlink(Direction::Send), unlink(Direction::Receive)]);
        assert!(describe(&project, &both)
            .unwrap()
            .starts_with("Unlinked object 3 of"));
    }

    #[test]
    fn anything_else_keeps_its_command_name() {
        let project = project(switch(), None);
        // Unknown ids: the command itself will refuse; no invented sentence.
        let missing = Command::LinkComObject {
            com_object: ComObjectInstanceId(999),
            ga: GroupAddressId(7),
            direction: Direction::Send,
        };
        assert_eq!(describe(&project, &missing), None);
        let mixed = Command::Batch(vec![link(Direction::Send), unlink(Direction::Receive)]);
        assert_eq!(describe(&project, &mixed), None);
        assert_eq!(
            describe(
                &project,
                &Command::RenameGroupAddress {
                    id: GroupAddressId(7),
                    name: "x".into()
                }
            ),
            None
        );
    }

    #[test]
    fn parts_read_as_a_list() {
        let parts = |a, b, c, d| ProgrammingParts {
            address: a,
            application: b,
            parameters: c,
            group_addresses: d,
        };
        assert_eq!(parts_text(ProgrammingParts::NONE), "");
        assert_eq!(parts_text(parts(false, false, true, false)), "parameters");
        assert_eq!(
            parts_text(parts(true, false, false, true)),
            "address and group addresses"
        );
        assert_eq!(
            parts_text(ProgrammingParts::ALL),
            "address, application, parameters and group addresses"
        );
    }

    #[test]
    fn a_programming_change_reads_as_what_was_marked() {
        let project = project(switch(), IndividualAddress::new(1, 1, 5).ok());
        let mark = |change| Command::SetDeviceProgramming {
            device: DeviceId(1),
            change,
        };
        assert_eq!(
            describe(
                &project,
                &mark(ProgrammingChange::mark(ProgrammingParts::ALL, true))
            )
            .unwrap(),
            "“Kitchen actuator” (1.1.5) marked as programmed"
        );
        assert_eq!(
            describe(
                &project,
                &mark(ProgrammingChange::mark(ProgrammingParts::ALL, false))
            )
            .unwrap(),
            "“Kitchen actuator” (1.1.5) marked as not programmed"
        );
        let mixed = ProgrammingChange {
            address: Some(true),
            parameters: Some(false),
            ..ProgrammingChange::default()
        };
        assert_eq!(
            describe(&project, &mark(mixed)).unwrap(),
            "“Kitchen actuator” (1.1.5): address marked as programmed; \
             “Kitchen actuator” (1.1.5): parameters marked as not programmed"
        );
        assert_eq!(
            describe(&project, &mark(ProgrammingChange::default())),
            None
        );
        let missing = Command::SetDeviceProgramming {
            device: DeviceId(9),
            change: ProgrammingChange::mark(ProgrammingParts::ALL, true),
        };
        assert_eq!(describe(&project, &missing), None);
    }
}
