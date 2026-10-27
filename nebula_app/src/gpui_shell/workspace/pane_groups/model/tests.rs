use super::*;

fn directory(name: &str) -> ProjectDirectory {
    ProjectDirectory::new(&std::env::temp_dir().join("pebrel-pane-groups").join(name)).unwrap()
}

fn project(name: &str) -> PaneProject {
    PaneProject::fixed(directory(name))
}

#[test]
fn same_directory_reuses_a_group_and_cd_does_not_reassign_it() {
    let a = project("a");
    let mut groups = PaneGroups::default();
    groups.sync([(11, a.clone()), (12, project("a")), (13, project("b"))]);
    assert_eq!(groups.groups().len(), 2);
    assert_eq!(groups.groups()[0].panes, [11, 12]);
    assert!(!a.observe_initial_directory(directory("b").path()));
    groups.sync([(11, a), (12, project("a")), (13, project("b"))]);
    assert_eq!(groups.groups()[0].directory, directory("a"));
    assert_eq!(groups.groups()[0].panes, [11, 12]);
}

#[test]
fn an_explicit_or_inherited_project_wins_over_spawn_directory() {
    let explicit = project("fixed");
    let inherited = explicit.clone();
    assert!(!inherited.observe_initial_directory(directory("elsewhere").path()));
    assert_eq!(inherited.directory(), Some(&directory("fixed")));
    let mut groups = PaneGroups::default();
    groups.sync([(1, explicit), (2, inherited)]);
    assert_eq!(groups.groups()[0].panes, [1, 2]);
}

#[test]
fn pending_splits_share_the_first_valid_directory_even_if_the_parent_closes() {
    let parent = PaneProject::default();
    let child = parent.clone();
    let mut groups = PaneGroups::default();
    groups.sync([(1, parent.clone()), (2, child.clone())]);
    assert_eq!(groups.pending_panes().collect::<Vec<_>>(), [1, 2]);
    assert!(!parent.observe_initial_directory(Path::new("not-absolute")));
    assert!(parent.directory().is_none());
    groups.sync([(2, child.clone())]);
    assert!(child.observe_initial_directory(directory("resolved").path()));
    assert!(groups.needs_refresh());
    assert!(!parent.observe_initial_directory(directory("other").path()));
    groups.sync([(2, child)]);
    assert!(!groups.needs_refresh());
    assert_eq!(groups.groups()[0].directory, directory("resolved"));
    assert_eq!(groups.groups()[0].panes, [2]);
    assert_eq!(groups.pending_panes().count(), 0);
}

#[test]
fn tab_order_and_focus_changes_do_not_reorder_existing_groups_or_panes() {
    let a = project("a");
    let b = project("b");
    let mut groups = PaneGroups::default();
    groups.sync([(10, a.clone()), (20, b.clone()), (30, a.clone())]);
    groups.sync([(30, a.clone()), (20, b.clone()), (40, b.clone()), (10, a.clone())]);
    assert_eq!(groups.groups()[0].directory, directory("a"));
    assert_eq!(groups.groups()[0].panes, [10, 30]);
    assert_eq!(groups.groups()[1].panes, [20, 40]);
    groups.sync([(10, a), (20, b)]);
    assert_eq!(groups.groups()[0].panes, [10]);
    assert_eq!(groups.groups()[1].panes, [20]);
}

#[test]
fn closing_the_last_pane_removes_its_group_and_reopening_appends() {
    let mut groups = PaneGroups::default();
    groups.sync([(1, project("a")), (2, project("b"))]);
    groups.sync([(2, project("b"))]);
    assert_eq!(groups.groups().len(), 1);
    groups.sync([(3, project("a")), (2, project("b"))]);
    assert_eq!(groups.groups()[0].directory, directory("b"));
    assert_eq!(groups.groups()[1].panes, [3]);
    groups.sync([]);
    assert!(groups.groups().is_empty());
    assert_eq!(groups.pending_panes().count(), 0);
}

#[test]
fn transfer_preserves_fixed_membership_and_appends_to_the_receiving_window() {
    let moved = project("original");
    let mut source = PaneGroups::default();
    let mut target = PaneGroups::default();
    source.sync([(1, moved.clone())]);
    target.sync([(2, project("target"))]);
    source.sync([]);
    target.sync([(1, moved), (2, project("target"))]);
    assert!(source.groups().is_empty());
    assert_eq!(target.groups()[0].panes, [2]);
    assert_eq!(target.groups()[1].directory, directory("original"));
    assert_eq!(target.groups()[1].panes, [1]);
}

#[test]
fn later_resolution_adds_a_group_after_groups_already_visible() {
    let pending = PaneProject::default();
    let mut groups = PaneGroups::default();
    groups.sync([(1, pending.clone()), (2, project("known"))]);
    assert!(pending.observe_initial_directory(directory("late").path()));
    groups.sync([(1, pending), (2, project("known"))]);
    assert_eq!(groups.groups()[0].directory, directory("known"));
    assert_eq!(groups.groups()[1].panes, [1]);
}

#[test]
fn unresolved_lineage_notifies_both_window_projections_without_a_cwd_change() {
    let pending = PaneProject::default();
    let mut source = PaneGroups::default();
    let mut target = PaneGroups::default();
    source.sync([(1, pending.clone())]);
    target.sync([(2, pending.clone())]);
    assert!(pending.observe_initial_directory(directory("first-report").path()));
    assert!(source.needs_refresh());
    assert!(target.needs_refresh());
    source.sync([(1, pending.clone())]);
    target.sync([(2, pending)]);
    assert_eq!(source.groups()[0].directory, target.groups()[0].directory);
    assert!(!source.needs_refresh());
    assert!(!target.needs_refresh());
}
