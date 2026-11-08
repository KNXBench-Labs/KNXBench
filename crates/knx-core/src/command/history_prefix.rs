//! Proves which bottom undo/redo states two editor histories share, for incremental storage.

use super::{CommandError, CommandStack, HistoryEntry};
use crate::Project;

/// Bottom states per side (positions `0..undo` and `0..redo`) that two
/// histories provably share. Zero means nothing may be reused on that side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SharedHistoryPrefix {
    pub undo: usize,
    pub redo: usize,
}

impl CommandStack {
    /// How many bottom states per side this history over the live `project`
    /// shares with `earlier` over `earlier_project`. A prefix counts only when
    /// its entries are equal and replaying each history down to the prefix
    /// boundary yields equal projects: replay is deterministic, so every state
    /// below is then equal too. A direct project edit outside the stack changes
    /// the boundary and answers zero. Any replay failure also answers zero.
    /// Only clones are replayed; neither history nor project changes.
    pub fn shared_snapshot_prefix(
        &self,
        project: &Project,
        earlier: &CommandStack,
        earlier_project: &Project,
    ) -> SharedHistoryPrefix {
        SharedHistoryPrefix {
            undo: shared_side(&self.undo, project, &earlier.undo, earlier_project),
            redo: shared_side(&self.redo, project, &earlier.redo, earlier_project),
        }
    }
}

fn shared_side(
    entries: &[HistoryEntry],
    project: &Project,
    earlier: &[HistoryEntry],
    earlier_project: &Project,
) -> usize {
    let shared = entries
        .iter()
        .zip(earlier)
        .take_while(|(entry, earlier)| entry == earlier)
        .count();
    if shared == 0 {
        return 0;
    }
    match (
        boundary(&entries[shared..], project),
        boundary(&earlier[shared..], earlier_project),
    ) {
        (Ok(state), Ok(earlier_state)) if state == earlier_state => shared,
        _ => 0,
    }
}

/// The state just above the shared prefix: `above` replayed top-down, exactly
/// as `CommandStack::visit_snapshot_states` walks one side.
fn boundary(above: &[HistoryEntry], project: &Project) -> Result<Project, CommandError> {
    let mut current = project.clone();
    for entry in above.iter().rev() {
        entry.clone().apply(&mut current)?;
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::SharedHistoryPrefix;
    use crate::command::{Command, CommandStack, HistorySide};
    use crate::{DeviceId, IndividualAddress, Project};

    fn address(octet: u8) -> Command {
        Command::SetIndividualAddress {
            device: DeviceId(1),
            address: Some(IndividualAddress::new(1, 1, octet).unwrap()),
        }
    }

    fn edited(octets: &[u8]) -> (Project, CommandStack) {
        let mut project = super::super::tests::test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        for octet in octets {
            stack.do_command(&mut project, address(*octet)).unwrap();
        }
        (project, stack)
    }

    /// Every state the incremental writer keeps must equal the full replay.
    fn assert_prefix_states_equal(
        (project, stack): (&Project, &CommandStack),
        (earlier_project, earlier): (&Project, &CommandStack),
        shared: SharedHistoryPrefix,
    ) {
        let states = |project: &Project, stack: &CommandStack| {
            let mut states = Vec::new();
            stack
                .visit_snapshot_states(project, |side, position, state| {
                    states.push((side, position, state.clone()));
                    true
                })
                .unwrap();
            states
        };
        let now = states(project, stack);
        let before = states(earlier_project, earlier);
        for (side, limit) in [
            (HistorySide::Undo, shared.undo),
            (HistorySide::Redo, shared.redo),
        ] {
            for position in 0..limit {
                let pick = |all: &[(HistorySide, usize, Project)]| {
                    all.iter()
                        .find(|(s, p, _)| *s == side && *p == position)
                        .map(|(_, _, state)| state.clone())
                        .unwrap()
                };
                assert_eq!(pick(&now), pick(&before), "{side:?} {position}");
            }
        }
    }

    #[test]
    fn a_new_edit_keeps_every_earlier_undo_state() {
        let (earlier_project, earlier) = edited(&[1, 2, 3]);
        let (mut project, mut stack) = (earlier_project.clone(), earlier.clone());
        stack.do_command(&mut project, address(4)).unwrap();
        let shared = stack.shared_snapshot_prefix(&project, &earlier, &earlier_project);
        assert_eq!(shared, SharedHistoryPrefix { undo: 3, redo: 0 });
        assert_prefix_states_equal((&project, &stack), (&earlier_project, &earlier), shared);
    }

    #[test]
    fn undo_and_redo_keep_the_untouched_bottom_of_both_sides() {
        let (earlier_project, earlier) = edited(&[1, 2, 3]);
        let (mut project, mut stack) = (earlier_project.clone(), earlier.clone());
        stack.undo(&mut project).unwrap();
        let shared = stack.shared_snapshot_prefix(&project, &earlier, &earlier_project);
        assert_eq!(shared, SharedHistoryPrefix { undo: 2, redo: 0 });
        assert_prefix_states_equal((&project, &stack), (&earlier_project, &earlier), shared);

        let (undone_project, undone) = (project.clone(), stack.clone());
        stack.undo(&mut project).unwrap();
        let shared = stack.shared_snapshot_prefix(&project, &undone, &undone_project);
        assert_eq!(shared, SharedHistoryPrefix { undo: 1, redo: 1 });
        assert_prefix_states_equal((&project, &stack), (&undone_project, &undone), shared);

        let (twice_project, twice) = (project.clone(), stack.clone());
        stack.redo(&mut project).unwrap();
        let shared = stack.shared_snapshot_prefix(&project, &twice, &twice_project);
        assert_eq!(shared, SharedHistoryPrefix { undo: 1, redo: 1 });
        assert_prefix_states_equal((&project, &stack), (&twice_project, &twice), shared);
    }

    #[test]
    fn an_edit_outside_the_stack_shares_nothing() {
        let (earlier_project, earlier) = edited(&[1, 2]);
        let (mut project, mut stack) = (earlier_project.clone(), earlier.clone());
        // A direct change survives every replay down the stack, so no stored
        // state below it may be kept.
        project.info.name = "changed outside the history".into();
        stack.do_command(&mut project, address(3)).unwrap();
        let shared = stack.shared_snapshot_prefix(&project, &earlier, &earlier_project);
        assert_eq!(shared, SharedHistoryPrefix::default());
    }

    #[test]
    fn rewritten_entries_and_foreign_histories_share_nothing() {
        let (earlier_project, earlier) = edited(&[1, 2]);
        let (project, stack) = edited(&[5, 2]);
        assert_eq!(
            stack.shared_snapshot_prefix(&project, &earlier, &earlier_project),
            SharedHistoryPrefix::default()
        );
        let empty = CommandStack::new();
        assert_eq!(
            empty.shared_snapshot_prefix(&earlier_project, &earlier, &earlier_project),
            SharedHistoryPrefix::default()
        );
    }

    #[test]
    fn recovered_snapshot_entries_are_shared_like_inverses() {
        let (project, stack) = edited(&[1, 2]);
        let (undo, redo) = stack.snapshot_states(&project).unwrap();
        let earlier = CommandStack::from_snapshot_states(undo, redo);
        let (mut current, mut stack) = (project.clone(), earlier.clone());
        stack.do_command(&mut current, address(3)).unwrap();
        let shared = stack.shared_snapshot_prefix(&current, &earlier, &project);
        assert_eq!(shared, SharedHistoryPrefix { undo: 2, redo: 0 });
        assert_prefix_states_equal((&current, &stack), (&project, &earlier), shared);
    }
}
