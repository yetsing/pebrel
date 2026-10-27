use super::model::PaneProject;
use super::*;
use crate::platform::project_directory::ProjectDirectory;
use crate::session::{LaunchSession, LayoutSession, SplitAxis, TabSession};
use gpui::{TestAppContext, VisualTestContext};

fn fixture(cx: &mut TestAppContext) -> (Entity<NebulaWorkspace>, VisualTestContext) {
    let hub = crate::runtime_api::RuntimeHub::new();
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::gpui_shell::math_view::register(cx);
        crate::gpui_shell::file_editor::init(cx);
        super::super::init(cx);
        windowing::initialize(cx, hub.clone());
        cx.set_reduce_motion(true);
    });
    let mut result = None;
    let (_, window) = cx.add_window_view(|window, cx| {
        let workspace = cx.new(|cx| {
            NebulaWorkspace::new(
                window,
                None,
                None,
                1,
                hub,
                windowing::WorkspaceStartup::Empty,
                windowing::WindowRole::Regular,
                cx,
            )
        });
        result = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    (result.unwrap(), window.clone())
}

fn launch() -> LaunchSession {
    LaunchSession::Shell {
        name: "test".into(),
        program: "pebrel-test-missing-shell-executable".into(),
        args: vec![],
    }
}

fn add_tab(
    workspace: &mut NebulaWorkspace,
    cwd: Option<std::path::PathBuf>,
    project: Option<PaneProject>,
    window: &mut Window,
    cx: &mut Context<NebulaWorkspace>,
) -> u64 {
    workspace.add_terminal_for_project(launch(), cwd, None, project, window, cx)
}

#[gpui::test]
fn explicit_project_split_global_new_and_close_use_live_pane_ids(cx: &mut TestAppContext) {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let directory = ProjectDirectory::new(a.path()).unwrap();
    let (workspace, mut window) = fixture(cx);
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            let first = add_tab(
                workspace,
                Some(b.path().into()),
                Some(PaneProject::fixed(directory.clone())),
                window,
                cx,
            );
            let original = workspace.tabs[workspace.active].focused_view().unwrap().clone();
            assert_eq!(workspace.pane_groups.groups()[0].directory, directory);
            assert_eq!(original.read(cx).cwd, b.path().to_string_lossy());
            let second = workspace.split_focused(SplitDirection::LeftRight, window, cx).unwrap();
            assert_eq!(workspace.pane_groups.groups()[0].panes, [first, second]);
            workspace.add_terminal(window, cx);
            let third = workspace.active_terminal_pane_id().unwrap();
            assert_eq!(workspace.pane_groups.groups()[0].panes, [first, second, third]);
            let other = add_tab(workspace, Some(b.path().into()), None, window, cx);
            assert_eq!(workspace.pane_groups.groups().len(), 2);
            workspace.move_tab(workspace.tab_of_pane(other).unwrap(), 0, window, cx);
            assert_eq!(workspace.pane_groups.groups()[0].panes, [first, second, third]);
            workspace.focus_pane(workspace.tab_of_pane(first).unwrap(), first, window, cx);
            workspace.close_pane(workspace.tab_of_pane(second).unwrap(), second, window, cx);
            assert_eq!(workspace.pane_groups.groups()[0].panes, [first, third]);
            workspace.close_tab(workspace.tab_of_pane(first).unwrap(), window, cx);
            assert_eq!(workspace.pane_groups.groups()[0].panes, [third]);
            // A late cwd/title event from the removed entity must not reinsert its pane.
            original.update(cx, |view, _| view.cwd = a.path().to_string_lossy().into_owned());
            workspace.on_terminal_event(&original, &TerminalViewEvent::TitleChanged, window, cx);
            assert_eq!(workspace.pane_groups.groups()[0].panes, [third]);
            workspace.close_tab(workspace.tab_of_pane(third).unwrap(), window, cx);
            assert_eq!(workspace.pane_groups.groups().len(), 1);
            assert_eq!(workspace.pane_groups.groups()[0].panes, [other]);
        })
    });
}

#[gpui::test]
fn first_cwd_report_resolves_pending_children_and_later_cd_keeps_membership(
    cx: &mut TestAppContext,
) {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let (workspace, mut window) = fixture(cx);
    let (first, second, child) = window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            let pending = PaneProject::default();
            let first = add_tab(workspace, None, Some(pending.clone()), window, cx);
            let second = add_tab(workspace, None, Some(pending), window, cx);
            assert!(workspace.pane_groups.groups().is_empty());
            assert_eq!(workspace.pane_groups.pending_panes().collect::<Vec<_>>(), [first, second]);
            let child = workspace.tabs[workspace.active].focused_view().unwrap().clone();
            (first, second, child)
        })
    });
    window.update(|_, cx| {
        child.update(cx, |view, cx| {
            view.cwd = a.path().to_string_lossy().into_owned();
            cx.emit(TerminalViewEvent::TitleChanged);
        })
    });
    cx.run_until_parked();
    workspace.read_with(&window, |workspace, _| {
        assert_eq!(workspace.pane_groups.groups()[0].panes, [first, second]);
        assert_eq!(workspace.pane_groups.pending_panes().count(), 0);
    });
    window.update(|_, cx| {
        child.update(cx, |view, cx| {
            view.cwd = b.path().to_string_lossy().into_owned();
            cx.emit(TerminalViewEvent::TitleChanged);
        })
    });
    cx.run_until_parked();
    workspace.read_with(&window, |workspace, _| {
        assert_eq!(
            workspace.pane_groups.groups()[0].directory,
            ProjectDirectory::new(a.path()).unwrap()
        );
        assert_eq!(workspace.pane_groups.groups()[0].panes, [first, second]);
    });
}

#[gpui::test]
fn duplicate_keeps_each_panes_project_but_cold_restore_groups_by_initial_directory(
    cx: &mut TestAppContext,
) {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let directory = ProjectDirectory::new(a.path()).unwrap();
    let leaf = || LayoutSession::Pane {
        cwd: b.path().to_string_lossy().into_owned(),
        launch: Some(launch()),
        custom_name: None,
        agent: None,
    };
    let saved = TabSession {
        cwd: String::new(),
        custom_name: None,
        color: None,
        launch: Some(launch()),
        layout: Some(LayoutSession::Split {
            axis: SplitAxis::LeftRight,
            ratio_permille: 500,
            first: Box::new(leaf()),
            second: Box::new(leaf()),
        }),
        active_pane: 1,
    };
    let (workspace, mut window) = fixture(cx);
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            assert!(workspace.restore_tab(&saved, false, window, cx));
            assert_eq!(
                workspace.pane_groups.groups()[0].directory,
                ProjectDirectory::new(b.path()).unwrap()
            );
            let WorkspaceTab::Terminal { panes, .. } = &mut workspace.tabs[0] else { panic!() };
            // A split tab may contain panes from different projects after docking.
            panes[0].project = Some(PaneProject::fixed(directory.clone()));
            panes.reverse();
            workspace.sync_pane_groups();
            let first = workspace
                .pane_groups
                .groups()
                .iter()
                .find(|g| g.directory == directory)
                .unwrap()
                .panes[0];
            workspace.duplicate_tab(0, window, cx);
            let duplicate = workspace.active;
            let WorkspaceTab::Terminal { panes, tree, .. } = &workspace.tabs[duplicate] else {
                panic!()
            };
            assert_eq!(panes.len(), 2);
            let copied_first = panes.iter().find(|pane| pane.id == tree.leaves()[0]).unwrap();
            assert_ne!(copied_first.id, first);
            assert_eq!(copied_first.project.as_ref().unwrap().directory(), Some(&directory));
            assert_eq!(copied_first.view.read(cx).cwd, b.path().to_string_lossy());
            assert_eq!(workspace.pane_groups.groups().len(), 2);
            assert_eq!(
                workspace
                    .pane_groups
                    .groups()
                    .iter()
                    .find(|g| g.directory == directory)
                    .unwrap()
                    .panes,
                [first, copied_first.id]
            );
        })
    });
}
