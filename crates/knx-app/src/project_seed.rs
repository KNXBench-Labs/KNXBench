//! Starting structure for a brand-new project, applied atomically through core commands.
//!
//! The new-project wizard (ADR-0093) lets a user describe areas and lines,
//! a building tree and a main/middle group-range skeleton before the project
//! exists. This module turns that description into ordinary
//! [`knx_core::Command`]s and applies them to the not-yet-installed project,
//! so every rule the explorer's own create commands obey (duplicate area or
//! line numbers, overlapping or badly nested ranges, ID allocation) is the
//! same rule here. No KNX structure is invented: only what the seed names is
//! created.
//!
//! The application is all-or-nothing. Every node is checked and applied on a
//! private copy; the caller's project changes only if the whole seed
//! succeeded. A refusal names the offending node by its wire path
//! (`areas[0].lines[1]`) so the client can point at it.

use std::fmt;

use knx_core::{
    Area, BuildingPart, BuildingPartType, Command, CompletionStatus, GroupAddress,
    GroupAddressStyle, GroupRange, IndividualAddress, InstallationId, Line, Project, SourceRef,
};

/// Upper bound on the number of structure nodes (areas, lines, building
/// parts, main and middle ranges together) one seed may describe. It is a
/// guard against oversized requests, checked before any ID is allocated,
/// not a statement about what a KNX installation may contain.
pub const MAX_SEED_NODES: usize = 2000;

/// What a new project should start with. Every list may be empty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectSeed {
    pub areas: Vec<SeedArea>,
    pub buildings: Vec<SeedBuildingPart>,
    pub group_ranges: Vec<SeedMainRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedArea {
    pub name: String,
    /// Area number, 0–15 (the high nibble of an individual address).
    pub address: u8,
    pub lines: Vec<SeedLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedLine {
    pub name: String,
    /// Line number inside its area, 0–15.
    pub address: u8,
    /// Opaque `MediumTypeRefId`, stored as given (the explorer's default is
    /// `MT-0`). Not interpreted, exactly like an imported line's.
    pub medium_ref: String,
}

/// The building-part kinds the wizard offers. The explorer keeps the full
/// [`BuildingPartType`] list; the wizard deliberately does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedBuildingKind {
    Building,
    Floor,
    Room,
    DistributionBoard,
}

impl SeedBuildingKind {
    fn core_type(self) -> BuildingPartType {
        match self {
            SeedBuildingKind::Building => BuildingPartType::Building,
            SeedBuildingKind::Floor => BuildingPartType::Floor,
            SeedBuildingKind::Room => BuildingPartType::Room,
            SeedBuildingKind::DistributionBoard => BuildingPartType::DistributionBoard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedBuildingPart {
    pub name: String,
    pub kind: SeedBuildingKind,
    pub children: Vec<SeedBuildingPart>,
}

/// A main group range: `main/0/0`–`main/7/255` in three-level style,
/// `main/0`–`main/2047` in two-level style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedMainRange {
    pub name: String,
    /// Main group number, 0–31.
    pub main: u8,
    /// Middle ranges; three-level style only.
    pub middles: Vec<SeedMiddleRange>,
}

/// A middle group range `main/middle/0`–`main/middle/255`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedMiddleRange {
    pub name: String,
    /// Middle group number, 0–7.
    pub middle: u8,
}

/// Why a seed was refused. Nothing was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedError {
    /// Wire path of the offending node, empty for a seed-wide refusal.
    pub path: String,
    pub reason: String,
}

impl SeedError {
    fn at(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for SeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.reason)
        } else {
            write!(f, "{}: {}", self.path, self.reason)
        }
    }
}

impl std::error::Error for SeedError {}

/// What a successfully applied seed created, for the session log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeedSummary {
    pub areas: usize,
    pub lines: usize,
    pub building_parts: usize,
    pub group_ranges: usize,
}

impl SeedSummary {
    pub fn is_empty(&self) -> bool {
        *self == SeedSummary::default()
    }
}

impl ProjectSeed {
    /// Number of structure nodes this seed describes.
    pub fn node_count(&self) -> usize {
        fn building(part: &SeedBuildingPart) -> usize {
            1 + part.children.iter().map(building).sum::<usize>()
        }
        self.areas.iter().map(|a| 1 + a.lines.len()).sum::<usize>()
            + self.buildings.iter().map(building).sum::<usize>()
            + self
                .group_ranges
                .iter()
                .map(|r| 1 + r.middles.len())
                .sum::<usize>()
    }
}

/// Applies `seed` to `installation` of `project`, all or nothing.
///
/// On `Err` the project is exactly as it was. The project's group-address
/// style decides how ranges are spelled; a `Free`-style project accepts no
/// ranges at all, a two-level one no middle ranges.
pub fn apply_project_seed(
    project: &mut Project,
    installation: InstallationId,
    seed: &ProjectSeed,
) -> Result<SeedSummary, SeedError> {
    let count = seed.node_count();
    if count > MAX_SEED_NODES {
        return Err(SeedError::at(
            "",
            format!(
                "the starting structure has {count} nodes, more than the {MAX_SEED_NODES} allowed"
            ),
        ));
    }
    if !project.installations.iter().any(|i| i.id == installation) {
        return Err(SeedError::at(
            "",
            format!("installation {} does not exist", installation.0),
        ));
    }
    validate(seed, project.info.group_address_style)?;

    let mut candidate = project.clone();
    let mut summary = SeedSummary::default();
    let target = Some(installation);

    for (a, area) in seed.areas.iter().enumerate() {
        let path = format!("areas[{a}]");
        let id = candidate
            .ids
            .next_area_id()
            .map_err(|e| SeedError::at(&path, e.to_string()))?;
        apply(
            &mut candidate,
            &path,
            Command::CreateArea {
                area: Area {
                    id,
                    source: kb_source("Area", id.0),
                    name: area.name.trim().to_string(),
                    address: area.address,
                    completion: CompletionStatus::Editing,
                    lines: vec![],
                },
                installation: target,
            },
        )?;
        summary.areas += 1;
        for (l, line) in area.lines.iter().enumerate() {
            let path = format!("{path}.lines[{l}]");
            let line_id = candidate
                .ids
                .next_line_id()
                .map_err(|e| SeedError::at(&path, e.to_string()))?;
            apply(
                &mut candidate,
                &path,
                Command::CreateLine {
                    area: id,
                    line: Line {
                        id: line_id,
                        source: kb_source("Line", line_id.0),
                        name: line.name.trim().to_string(),
                        address: line.address,
                        medium_ref: line.medium_ref.trim().to_string(),
                        domain_address: None,
                        domain_address_is_checked: None,
                        ip_routing_multicast_address: None,
                        multicast_ttl: None,
                        completion: CompletionStatus::Editing,
                        devices: vec![],
                    },
                },
            )?;
            summary.lines += 1;
        }
    }

    for (b, part) in seed.buildings.iter().enumerate() {
        apply_building(
            &mut candidate,
            target,
            &format!("buildings[{b}]"),
            part,
            None,
            &mut summary,
        )?;
    }

    let style = candidate.info.group_address_style;
    for (r, main) in seed.group_ranges.iter().enumerate() {
        let path = format!("groupRanges[{r}]");
        let (start, end) = main_bounds(style, main.main).map_err(|e| SeedError::at(&path, e))?;
        let id = candidate
            .ids
            .next_group_range_id()
            .map_err(|e| SeedError::at(&path, e.to_string()))?;
        apply(
            &mut candidate,
            &path,
            Command::CreateGroupRange {
                range: GroupRange {
                    id,
                    source: kb_source("Range", id.0),
                    name: main.name.trim().to_string(),
                    start,
                    end,
                    parent: None,
                    children: vec![],
                },
                installation: target,
            },
        )?;
        summary.group_ranges += 1;
        for (m, middle) in main.middles.iter().enumerate() {
            let path = format!("{path}.middles[{m}]");
            let (start, end) =
                middle_bounds(main.main, middle.middle).map_err(|e| SeedError::at(&path, e))?;
            let middle_id = candidate
                .ids
                .next_group_range_id()
                .map_err(|e| SeedError::at(&path, e.to_string()))?;
            apply(
                &mut candidate,
                &path,
                Command::CreateGroupRange {
                    range: GroupRange {
                        id: middle_id,
                        source: kb_source("Range", middle_id.0),
                        name: middle.name.trim().to_string(),
                        start,
                        end,
                        parent: Some(id),
                        children: vec![],
                    },
                    installation: None,
                },
            )?;
            summary.group_ranges += 1;
        }
    }

    *project = candidate;
    Ok(summary)
}

fn apply_building(
    project: &mut Project,
    installation: Option<InstallationId>,
    path: &str,
    part: &SeedBuildingPart,
    parent: Option<knx_core::BuildingPartId>,
    summary: &mut SeedSummary,
) -> Result<(), SeedError> {
    let id = project
        .ids
        .next_building_part_id()
        .map_err(|e| SeedError::at(path, e.to_string()))?;
    apply(
        project,
        path,
        Command::CreateBuildingPart {
            part: BuildingPart {
                id,
                source: kb_source("Building", id.0),
                name: part.name.trim().to_string(),
                number: None,
                kind: part.kind.core_type(),
                default_line: None,
                completion: CompletionStatus::Editing,
                children: vec![],
                devices: vec![],
                parent,
            },
            // A child follows its parent's installation; naming one again
            // would only give the core a second answer to compare.
            installation: if parent.is_none() { installation } else { None },
        },
    )?;
    summary.building_parts += 1;
    for (c, child) in part.children.iter().enumerate() {
        apply_building(
            project,
            installation,
            &format!("{path}.children[{c}]"),
            child,
            Some(id),
            summary,
        )?;
    }
    Ok(())
}

fn apply(project: &mut Project, path: &str, command: Command) -> Result<(), SeedError> {
    command
        .apply(project)
        .map(|_inverse| ())
        .map_err(|error| SeedError::at(path, error.to_string()))
}

/// Same provenance shape as the explorer's own create commands
/// (`KB-<Kind>-<id>`), so a seeded node is indistinguishable from one the
/// user added by hand afterwards.
fn kb_source(kind: &str, id: u32) -> SourceRef {
    let text = format!("KB-{kind}-{id}");
    SourceRef {
        path: text.clone(),
        ets_id: text,
    }
}

fn main_bounds(style: GroupAddressStyle, main: u8) -> Result<(GroupAddress, GroupAddress), String> {
    let (start, end) = match style {
        GroupAddressStyle::ThreeLevel => (format!("{main}/0/0"), format!("{main}/7/255")),
        GroupAddressStyle::TwoLevel => (format!("{main}/0"), format!("{main}/2047")),
        GroupAddressStyle::Free => {
            return Err("free-style projects have no main group ranges".to_string());
        }
    };
    Ok((
        GroupAddress::parse(&start, style).map_err(|e| e.to_string())?,
        GroupAddress::parse(&end, style).map_err(|e| e.to_string())?,
    ))
}

fn middle_bounds(main: u8, middle: u8) -> Result<(GroupAddress, GroupAddress), String> {
    let style = GroupAddressStyle::ThreeLevel;
    Ok((
        GroupAddress::parse(&format!("{main}/{middle}/0"), style).map_err(|e| e.to_string())?,
        GroupAddress::parse(&format!("{main}/{middle}/255"), style).map_err(|e| e.to_string())?,
    ))
}

/// Shape checks that need no project state. Uniqueness and nesting are left
/// to the core commands, which already know them.
fn validate(seed: &ProjectSeed, style: GroupAddressStyle) -> Result<(), SeedError> {
    for (a, area) in seed.areas.iter().enumerate() {
        let path = format!("areas[{a}]");
        require_name(&path, &area.name)?;
        IndividualAddress::new(area.address, 0, 0)
            .map_err(|e| SeedError::at(&path, e.to_string()))?;
        for (l, line) in area.lines.iter().enumerate() {
            let path = format!("{path}.lines[{l}]");
            require_name(&path, &line.name)?;
            IndividualAddress::new(area.address, line.address, 0)
                .map_err(|e| SeedError::at(&path, e.to_string()))?;
            if line.medium_ref.trim().is_empty() {
                return Err(SeedError::at(
                    &path,
                    "the medium reference must not be blank",
                ));
            }
        }
    }
    fn building(path: &str, part: &SeedBuildingPart) -> Result<(), SeedError> {
        require_name(path, &part.name)?;
        for (c, child) in part.children.iter().enumerate() {
            building(&format!("{path}.children[{c}]"), child)?;
        }
        Ok(())
    }
    for (b, part) in seed.buildings.iter().enumerate() {
        building(&format!("buildings[{b}]"), part)?;
    }
    for (r, main) in seed.group_ranges.iter().enumerate() {
        let path = format!("groupRanges[{r}]");
        require_name(&path, &main.name)?;
        main_bounds(style, main.main).map_err(|e| SeedError::at(&path, e))?;
        if !main.middles.is_empty() && style != GroupAddressStyle::ThreeLevel {
            return Err(SeedError::at(
                &path,
                "middle group ranges exist only in three-level style",
            ));
        }
        for (m, middle) in main.middles.iter().enumerate() {
            let path = format!("{path}.middles[{m}]");
            require_name(&path, &middle.name)?;
            middle_bounds(main.main, middle.middle).map_err(|e| SeedError::at(&path, e))?;
        }
    }
    Ok(())
}

fn require_name(path: &str, name: &str) -> Result<(), SeedError> {
    if name.trim().is_empty() {
        Err(SeedError::at(path, "the name must not be blank"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{Installation, Language, Topology};

    fn blank(style: GroupAddressStyle) -> Project {
        let mut project = Project::new(Language("en".into()));
        project.info.group_address_style = style;
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "Main".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::default(),
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        project
    }

    fn line(name: &str, address: u8) -> SeedLine {
        SeedLine {
            name: name.into(),
            address,
            medium_ref: "MT-0".into(),
        }
    }

    fn area(name: &str, address: u8, lines: Vec<SeedLine>) -> SeedArea {
        SeedArea {
            name: name.into(),
            address,
            lines,
        }
    }

    fn part(
        name: &str,
        kind: SeedBuildingKind,
        children: Vec<SeedBuildingPart>,
    ) -> SeedBuildingPart {
        SeedBuildingPart {
            name: name.into(),
            kind,
            children,
        }
    }

    fn full_seed() -> ProjectSeed {
        ProjectSeed {
            areas: vec![
                area("Area 1", 1, vec![line("Line 1.1", 1), line("Line 1.2", 2)]),
                area("Area 2", 2, vec![line("Line 2.1", 1)]),
            ],
            buildings: vec![part(
                "House",
                SeedBuildingKind::Building,
                vec![
                    part(
                        "Ground floor",
                        SeedBuildingKind::Floor,
                        vec![
                            part("Kitchen", SeedBuildingKind::Room, vec![]),
                            part("Board", SeedBuildingKind::DistributionBoard, vec![]),
                        ],
                    ),
                    part("Upper floor", SeedBuildingKind::Floor, vec![]),
                ],
            )],
            group_ranges: vec![SeedMainRange {
                name: "Lighting".into(),
                main: 1,
                middles: vec![
                    SeedMiddleRange {
                        name: "Ground floor".into(),
                        middle: 0,
                    },
                    SeedMiddleRange {
                        name: "Upper floor".into(),
                        middle: 1,
                    },
                ],
            }],
        }
    }

    #[test]
    fn a_full_seed_creates_exactly_what_it_names() {
        let mut project = blank(GroupAddressStyle::ThreeLevel);
        let summary = apply_project_seed(&mut project, InstallationId(0), &full_seed()).unwrap();
        assert_eq!(
            summary,
            SeedSummary {
                areas: 2,
                lines: 3,
                building_parts: 5,
                group_ranges: 3
            }
        );
        let installation = &project.installations[0];
        let areas: Vec<_> = installation
            .topology
            .areas
            .iter()
            .map(|a| (a.name.as_str(), a.address, a.lines.len()))
            .collect();
        assert_eq!(areas, vec![("Area 1", 1, 2), ("Area 2", 2, 1)]);
        let lines: Vec<_> = installation
            .topology
            .lines
            .iter()
            .map(|l| (l.name.as_str(), l.address, l.medium_ref.as_str()))
            .collect();
        assert_eq!(
            lines,
            vec![
                ("Line 1.1", 1, "MT-0"),
                ("Line 1.2", 2, "MT-0"),
                ("Line 2.1", 1, "MT-0")
            ]
        );

        let house = installation
            .buildings
            .iter()
            .find(|p| p.name == "House")
            .unwrap();
        assert_eq!(house.kind, BuildingPartType::Building);
        assert_eq!(house.parent, None);
        assert_eq!(house.children.len(), 2);
        let kitchen = installation
            .buildings
            .iter()
            .find(|p| p.name == "Kitchen")
            .unwrap();
        let ground = installation
            .buildings
            .iter()
            .find(|p| p.name == "Ground floor")
            .unwrap();
        assert_eq!(kitchen.parent, Some(ground.id));
        assert_eq!(ground.parent, Some(house.id));
        let board = installation
            .buildings
            .iter()
            .find(|p| p.name == "Board")
            .unwrap();
        assert_eq!(board.kind, BuildingPartType::DistributionBoard);

        let style = GroupAddressStyle::ThreeLevel;
        let lighting = installation
            .group_ranges
            .iter()
            .find(|r| r.name == "Lighting")
            .unwrap();
        assert_eq!(
            (lighting.start.format(style), lighting.end.format(style)),
            ("1/0/0".into(), "1/7/255".into())
        );
        let upper = installation
            .group_ranges
            .iter()
            .find(|r| r.name == "Upper floor")
            .unwrap();
        assert_eq!(upper.parent, Some(lighting.id));
        assert_eq!(
            (upper.start.format(style), upper.end.format(style)),
            ("1/1/0".into(), "1/1/255".into())
        );
        assert!(
            project.installations[0].group_addresses.is_empty(),
            "the seed never creates group addresses"
        );
    }

    #[test]
    fn names_are_stored_trimmed_with_the_explorers_provenance_shape() {
        let mut project = blank(GroupAddressStyle::ThreeLevel);
        let seed = ProjectSeed {
            areas: vec![area("  Area 1  ", 1, vec![])],
            ..ProjectSeed::default()
        };
        apply_project_seed(&mut project, InstallationId(0), &seed).unwrap();
        let area = &project.installations[0].topology.areas[0];
        assert_eq!(area.name, "Area 1");
        assert_eq!(area.source.ets_id, format!("KB-Area-{}", area.id.0));
        assert_eq!(area.completion, CompletionStatus::Editing);
    }

    #[test]
    fn an_empty_seed_changes_nothing() {
        let mut project = blank(GroupAddressStyle::ThreeLevel);
        let before = project.clone();
        let summary =
            apply_project_seed(&mut project, InstallationId(0), &ProjectSeed::default()).unwrap();
        assert!(summary.is_empty());
        assert_eq!(project, before);
    }

    #[test]
    fn two_level_main_ranges_span_the_whole_main_group() {
        let mut project = blank(GroupAddressStyle::TwoLevel);
        let seed = ProjectSeed {
            group_ranges: vec![SeedMainRange {
                name: "Heating".into(),
                main: 3,
                middles: vec![],
            }],
            ..ProjectSeed::default()
        };
        apply_project_seed(&mut project, InstallationId(0), &seed).unwrap();
        let range = &project.installations[0].group_ranges[0];
        let style = GroupAddressStyle::TwoLevel;
        assert_eq!(
            (range.start.format(style), range.end.format(style)),
            ("3/0".into(), "3/2047".into())
        );
    }

    /// Each refusal must leave the project byte-for-byte untouched, including
    /// the ID allocators: a failed seed reserves nothing.
    fn assert_refused(style: GroupAddressStyle, seed: ProjectSeed, path: &str, reason_part: &str) {
        let mut project = blank(style);
        let before = project.clone();
        let error = apply_project_seed(&mut project, InstallationId(0), &seed).unwrap_err();
        assert_eq!(error.path, path, "{error}");
        assert!(error.reason.contains(reason_part), "{error}");
        assert_eq!(project, before);
    }

    #[test]
    fn a_late_core_refusal_rolls_back_every_earlier_node() {
        let mut seed = full_seed();
        // Duplicate line number in area 2: the core refuses, after areas,
        // lines and nothing else were already applied to the candidate.
        seed.areas[1].lines.push(line("Line 2.1 again", 1));
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            seed,
            "areas[1].lines[1]",
            "line",
        );
    }

    #[test]
    fn duplicate_area_numbers_are_refused_by_the_core_rule() {
        let seed = ProjectSeed {
            areas: vec![area("A", 1, vec![]), area("B", 1, vec![])],
            ..ProjectSeed::default()
        };
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            seed,
            "areas[1]",
            "area address 1",
        );
    }

    #[test]
    fn duplicate_main_groups_are_refused_as_overlapping_ranges() {
        let seed = ProjectSeed {
            group_ranges: vec![
                SeedMainRange {
                    name: "A".into(),
                    main: 2,
                    middles: vec![],
                },
                SeedMainRange {
                    name: "B".into(),
                    main: 2,
                    middles: vec![],
                },
            ],
            ..ProjectSeed::default()
        };
        assert_refused(GroupAddressStyle::ThreeLevel, seed, "groupRanges[1]", "");
    }

    #[test]
    fn out_of_range_numbers_are_refused_before_anything_is_applied() {
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            ProjectSeed {
                areas: vec![area("A", 16, vec![])],
                ..ProjectSeed::default()
            },
            "areas[0]",
            "area 16 exceeds maximum 15",
        );
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            ProjectSeed {
                areas: vec![area("A", 1, vec![line("L", 16)])],
                ..ProjectSeed::default()
            },
            "areas[0].lines[0]",
            "line 16 exceeds maximum 15",
        );
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            ProjectSeed {
                group_ranges: vec![SeedMainRange {
                    name: "M".into(),
                    main: 32,
                    middles: vec![],
                }],
                ..ProjectSeed::default()
            },
            "groupRanges[0]",
            "main 32 exceeds maximum 31",
        );
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            ProjectSeed {
                group_ranges: vec![SeedMainRange {
                    name: "M".into(),
                    main: 1,
                    middles: vec![SeedMiddleRange {
                        name: "X".into(),
                        middle: 8,
                    }],
                }],
                ..ProjectSeed::default()
            },
            "groupRanges[0].middles[0]",
            "middle 8 exceeds maximum 7",
        );
    }

    #[test]
    fn blank_names_and_media_are_refused() {
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            ProjectSeed {
                buildings: vec![part(
                    "House",
                    SeedBuildingKind::Building,
                    vec![part("  ", SeedBuildingKind::Floor, vec![])],
                )],
                ..ProjectSeed::default()
            },
            "buildings[0].children[0]",
            "name must not be blank",
        );
        let mut blank_medium = line("L", 1);
        blank_medium.medium_ref = " ".into();
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            ProjectSeed {
                areas: vec![area("A", 1, vec![blank_medium])],
                ..ProjectSeed::default()
            },
            "areas[0].lines[0]",
            "medium reference",
        );
    }

    #[test]
    fn the_style_decides_which_ranges_are_allowed() {
        let main = |middles| SeedMainRange {
            name: "M".into(),
            main: 1,
            middles,
        };
        assert_refused(
            GroupAddressStyle::Free,
            ProjectSeed {
                group_ranges: vec![main(vec![])],
                ..ProjectSeed::default()
            },
            "groupRanges[0]",
            "free-style",
        );
        assert_refused(
            GroupAddressStyle::TwoLevel,
            ProjectSeed {
                group_ranges: vec![main(vec![SeedMiddleRange {
                    name: "X".into(),
                    middle: 0,
                }])],
                ..ProjectSeed::default()
            },
            "groupRanges[0]",
            "three-level",
        );
    }

    #[test]
    fn the_node_cap_refuses_before_any_id_is_reserved() {
        let floors = (0..MAX_SEED_NODES)
            .map(|i| part(&format!("F{i}"), SeedBuildingKind::Floor, vec![]))
            .collect();
        let seed = ProjectSeed {
            buildings: vec![part("House", SeedBuildingKind::Building, floors)],
            ..ProjectSeed::default()
        };
        assert_eq!(seed.node_count(), MAX_SEED_NODES + 1);
        assert_refused(
            GroupAddressStyle::ThreeLevel,
            seed,
            "",
            "more than the 2000 allowed",
        );
    }

    #[test]
    fn exactly_the_node_cap_is_accepted() {
        let rooms = (0..MAX_SEED_NODES - 1)
            .map(|i| part(&format!("R{i}"), SeedBuildingKind::Room, vec![]))
            .collect();
        let seed = ProjectSeed {
            buildings: vec![part("House", SeedBuildingKind::Building, rooms)],
            ..ProjectSeed::default()
        };
        let mut project = blank(GroupAddressStyle::ThreeLevel);
        let summary = apply_project_seed(&mut project, InstallationId(0), &seed).unwrap();
        assert_eq!(summary.building_parts, MAX_SEED_NODES);
    }

    #[test]
    fn an_unknown_installation_is_refused() {
        let mut project = blank(GroupAddressStyle::ThreeLevel);
        let before = project.clone();
        let error = apply_project_seed(&mut project, InstallationId(7), &full_seed()).unwrap_err();
        assert!(error.reason.contains("installation 7"), "{error}");
        assert_eq!(project, before);
    }
}
