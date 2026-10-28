use super::super::pane_groups::model::PaneProject;
use super::*;
use crate::session::LaunchSession;
use gpui::{Modifiers, TestAppContext, VisualTestContext};

fn fixture(cx: &mut TestAppContext) -> (Entity<NebulaWorkspace>, VisualTestContext) {
    let hub = crate::runtime_api::RuntimeHub::new();
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::gpui_shell::math_view::register(cx);
        crate::gpui_shell::file_editor::init(cx);
        super::super::init(cx);
        windowing::initialize(cx, hub.clone());
        cx.set_reduce_motion(true);
        let mut settings =
            crate::gpui_shell::config::Settings::load(nebula_settings::ThemeName::Nord);
        settings.shell_id = Some("cmd".into());
        cx.set_global(settings);
        cx.set_global(shell_launch::TestConfiguredLaunch(LaunchSession::Shell {
            name: "test".into(),
            program: "pebrel-test-missing-shell-executable".into(),
            args: vec![],
        }));
    });
    let mut result = None;
    let (_, window) = cx.add_window_view(|window, cx| {
        let workspace = cx.new(|cx| {
            let mut workspace = NebulaWorkspace::new(
                window,
                None,
                None,
                1,
                hub,
                windowing::WorkspaceStartup::Empty,
                windowing::WindowRole::Regular,
                cx,
            );
            workspace.tabs_position = nebula_settings::TabsPositionName::Sidebar;
            workspace.sidebar_width = 280.0;
            workspace.sidebar_collapsed = false;
            workspace.tabs_section_collapsed = false;
            workspace
        });
        result = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    window.simulate_resize(gpui::size(px(1100.0), px(700.0)));
    window.run_until_parked();
    let workspace = result.unwrap();
    workspace.update(&mut window.clone(), |workspace, cx| {
        workspace.tabs_position = nebula_settings::TabsPositionName::Sidebar;
        workspace.sidebar_width = 280.0;
        workspace.sidebar_collapsed = false;
        cx.notify();
    });
    (workspace, window.clone())
}

fn add(
    workspace: &mut NebulaWorkspace,
    path: &std::path::Path,
    window: &mut Window,
    cx: &mut Context<NebulaWorkspace>,
) -> u64 {
    workspace.add_terminal_for_project(
        LaunchSession::Shell {
            name: "test".into(),
            program: "pebrel-test-missing-shell-executable".into(),
            args: vec![],
        },
        Some(path.to_owned()),
        None,
        Some(PaneProject::fixed(ProjectDirectory::new(path).unwrap())),
        window,
        cx,
    )
}

fn draw(window: &mut VisualTestContext) {
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
}

fn bounds(
    window: &mut VisualTestContext,
    selector: impl Into<String>,
) -> gpui::Bounds<gpui::Pixels> {
    let selector = String::leak(selector.into());
    window.debug_bounds(selector).unwrap_or_else(|| panic!("missing control: {selector}"))
}

fn click(window: &mut VisualTestContext, selector: impl Into<String>) {
    let center = bounds(window, selector).center();
    window.simulate_mouse_move(center, None, Modifiers::default());
    window.simulate_click(center, Modifiers::default());
    draw(window);
}

#[gpui::test]
fn pane_controls_focus_exact_pane_after_reorder_unzoom_and_close_only_one(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (workspace, mut window) = fixture(cx);
    let (first, second, third) = window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            let first = add(workspace, directory.path(), window, cx);
            let second = workspace.split_focused(SplitDirection::LeftRight, window, cx).unwrap();
            if let WorkspaceTab::Terminal { zoomed, .. } = &mut workspace.tabs[0] {
                *zoomed = true;
            }
            let third = add(workspace, other.path(), window, cx);
            workspace.move_tab(0, 1, window, cx);
            (first, second, third)
        })
    });
    draw(&mut window);
    click(&mut window, format!("sidebar-pane-{first}"));
    let focus = workspace.read_with(&window, |workspace, cx| {
        let index = workspace.tab_of_pane(first).unwrap();
        assert_eq!(workspace.active, index);
        assert_eq!(workspace.active_terminal_pane_id(), Some(first));
        assert!(matches!(&workspace.tabs[index], WorkspaceTab::Terminal { zoomed: false, .. }));
        let view = workspace.tabs[index].focused_view().unwrap();
        view.read(cx).focus_handle(cx)
    });
    window.update(|window, _| assert!(focus.is_focused(window)));
    click(&mut window, format!("sidebar-pane-{second}"));
    assert_eq!(
        workspace.read_with(&window, |workspace, _| workspace.active_terminal_pane_id()),
        Some(second)
    );
    let row = bounds(&mut window, format!("sidebar-pane-{second}"));
    let close = bounds(&mut window, format!("sidebar-pane-close-{second}"));
    assert!(close.right() <= row.right() && close.left() >= row.left());
    click(&mut window, format!("sidebar-pane-close-{second}"));
    workspace.read_with(&window, |workspace, _| {
        assert!(workspace.tab_of_pane(first).is_some());
        assert!(workspace.tab_of_pane(second).is_none());
        assert!(workspace.tab_of_pane(third).is_some());
        assert_eq!(workspace.tabs.len(), 2);
    });
}

#[gpui::test]
fn picker_cancel_invalid_directory_and_reopen_reuse_fixed_project(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (workspace, mut window) = fixture(cx);
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            add(workspace, directory.path(), window, cx);
        })
    });
    draw(&mut window);
    click(&mut window, "sidebar-open-project");
    assert!(window.did_prompt_for_paths());
    click(&mut window, "sidebar-open-project");
    assert!(workspace.read_with(&window, |workspace, _| workspace.pane_sidebar.picker.is_some()));
    window.simulate_path_prompt_response(|options| {
        assert!(options.directories && !options.files && !options.multiple);
        None
    });
    draw(&mut window);
    assert_eq!(workspace.read_with(&window, |workspace, _| workspace.tabs.len()), 1);
    for (path, count, groups) in [
        (directory.path().to_owned(), 2, 1),
        (other.path().to_owned(), 3, 2),
        (directory.path().join("absent"), 3, 2),
    ] {
        click(&mut window, "sidebar-open-project");
        assert!(window.did_prompt_for_paths());
        window.simulate_path_prompt_response(move |_| Some(vec![path]));
        draw(&mut window);
        workspace.read_with(&window, |workspace, _| {
            assert!(workspace.pane_sidebar.picker.is_none());
            assert_eq!(workspace.tabs.len(), count);
            assert_eq!(workspace.pane_groups.groups().len(), groups);
        });
    }
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            while workspace.tabs.len() > 1 {
                workspace.close_tab(1, window, cx);
            }
        })
    });
}

#[gpui::test]
fn project_plus_uses_fixed_directory_global_plus_inherits_project_and_current_directory(
    cx: &mut TestAppContext,
) {
    let directory = tempfile::tempdir().unwrap();
    let current = tempfile::tempdir().unwrap();
    let (workspace, mut window) = fixture(cx);
    let first = window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            let first = add(workspace, directory.path(), window, cx);
            workspace.tabs[0]
                .focused_view()
                .unwrap()
                .update(cx, |view, _| view.cwd = current.path().to_string_lossy().into_owned());
            first
        })
    });
    draw(&mut window);
    click(&mut window, format!("sidebar-project-add-{}", directory.path().display()));
    workspace.read_with(&window, |workspace, cx| {
        assert_eq!(workspace.tabs.len(), 2);
        assert_eq!(
            workspace.tabs[workspace.active].focused_view().unwrap().read(cx).cwd,
            directory.path().to_string_lossy()
        );
        assert_eq!(workspace.pane_groups.groups().len(), 1);
    });
    click(&mut window, format!("sidebar-pane-{first}"));
    click(&mut window, "sidebar-new-tab");
    workspace.read_with(&window, |workspace, cx| {
        assert_eq!(workspace.tabs.len(), 3);
        assert_eq!(
            workspace.tabs[workspace.active].focused_view().unwrap().read(cx).cwd,
            current.path().to_string_lossy()
        );
        assert_eq!(workspace.pane_groups.groups().len(), 1);
        assert_eq!(workspace.pane_groups.groups()[0].panes.len(), 3);
    });
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            while workspace.tabs.len() > 1 {
                workspace.close_tab(1, window, cx);
            }
        })
    });
}

#[gpui::test]
fn mixed_height_scroll_hitboxes_follow_rows_and_narrow_close_stays_inside(cx: &mut TestAppContext) {
    let directories = (0..12).map(|_| tempfile::tempdir().unwrap()).collect::<Vec<_>>();
    let (workspace, mut window) = fixture(cx);
    let ids = window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.sidebar_width = 220.0;
            directories
                .iter()
                .map(|path| add(workspace, path.path(), window, cx))
                .collect::<Vec<_>>()
        })
    });
    draw(&mut window);
    draw(&mut window);
    let list = bounds(&mut window, "project-sidebar-list");
    let last = bounds(&mut window, format!("sidebar-pane-{}", ids[11]));
    assert!(
        last.top() >= list.top() && last.bottom() <= list.bottom(),
        "last={last:?}, list={list:?}"
    );
    click(&mut window, format!("sidebar-pane-{}", ids[10]));
    assert_eq!(
        workspace.read_with(&window, |workspace, _| workspace.active_terminal_pane_id()),
        Some(ids[10])
    );
    let close = bounds(&mut window, format!("sidebar-pane-close-{}", ids[10]));
    assert!(close.right() <= list.right());
    click(&mut window, format!("sidebar-pane-close-{}", ids[10]));
    workspace.read_with(&window, |workspace, _| {
        assert!(workspace.tab_of_pane(ids[10]).is_none());
        assert!(workspace.tab_of_pane(ids[11]).is_some());
        assert_eq!(workspace.tabs.len(), 11);
    });
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| workspace.activate_sidebar_pane(ids[0], window, cx))
    });
    draw(&mut window);
    draw(&mut window);
    let first = bounds(&mut window, format!("sidebar-pane-{}", ids[0]));
    assert!(first.top() >= list.top() && first.bottom() <= list.bottom());
    click(&mut window, format!("sidebar-pane-{}", ids[1]));
    assert_eq!(
        workspace.read_with(&window, |workspace, _| workspace.active_terminal_pane_id()),
        Some(ids[1])
    );
}

#[gpui::test]
fn document_entry_keeps_identity_after_reorder_and_settings_does_not_inherit_hidden_project(
    cx: &mut TestAppContext,
) {
    let directory = tempfile::tempdir().unwrap();
    let document = directory.path().join("document.md");
    std::fs::write(&document, "# Sidebar test").unwrap();
    let (workspace, mut window) = fixture(cx);
    let (pane, document_id) = window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            let pane = add(workspace, directory.path(), window, cx);
            workspace.open_document_path(document, window, cx);
            let id = workspace.tab_meta[workspace.active].runtime_id.clone();
            workspace.move_tab(1, 0, window, cx);
            workspace.activate_sidebar_pane(pane, window, cx);
            (pane, id)
        })
    });
    draw(&mut window);
    click(&mut window, format!("sidebar-document-{}", document_id.0));
    workspace.read_with(&window, |workspace, _| {
        assert_eq!(workspace.tab_meta[workspace.active].runtime_id, document_id);
        assert!(workspace.active_pane_project().is_none());
    });
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.activate_sidebar_pane(pane, window, cx);
            workspace.open_settings(window, cx);
            assert!(workspace.active_pane_project().is_none());
            workspace.add_terminal(window, cx);
            let new = workspace.active_terminal_pane_id().unwrap();
            assert!(!workspace.pane_groups.groups()[0].panes.contains(&new));
            workspace.close_pane(workspace.active, new, window, cx);
        })
    });
}

#[gpui::test]
fn unsupported_default_disables_project_creation_controls(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let (workspace, mut window) = fixture(cx);
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            add(workspace, directory.path(), window, cx);
            cx.global_mut::<shell_launch::TestConfiguredLaunch>().0 = LaunchSession::Shell {
                name: "WSL".into(),
                program: "C:\\pebrel-missing-test\\wsl.exe".into(),
                args: vec![],
            };
            cx.notify();
        })
    });
    draw(&mut window);
    click(&mut window, "sidebar-open-project");
    assert!(!window.did_prompt_for_paths());
    click(&mut window, format!("sidebar-project-add-{}", directory.path().display()));
    assert_eq!(workspace.read_with(&window, |workspace, _| workspace.tabs.len()), 1);
}

#[gpui::test]
fn explicit_tab_drag_resolves_reordered_source_and_keeps_project_panes(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (workspace, mut window) = fixture(cx);
    let (first, second, target) = window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            let first = add(workspace, directory.path(), window, cx);
            let second = workspace.split_focused(SplitDirection::LeftRight, window, cx).unwrap();
            let target = add(workspace, other.path(), window, cx);
            (first, second, target)
        })
    });
    draw(&mut window);
    let start = bounds(&mut window, format!("sidebar-tab-grip-{first}")).center();
    let end = bounds(&mut window, format!("sidebar-pane-{target}")).center();
    window.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    window.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            // Runtime/API operations can reorder tabs while the pointer gesture is pending.
            workspace.move_tab(0, 1, window, cx);
        })
    });
    window.simulate_mouse_move(
        gpui::point(start.x + px(10.0), start.y + px(10.0)),
        Some(MouseButton::Left),
        Modifiers::default(),
    );
    window.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
    window.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    draw(&mut window);
    workspace.read_with(&window, |workspace, _| {
        assert_eq!(workspace.tab_of_pane(first), Some(0));
        assert_eq!(workspace.tab_of_pane(second), Some(0));
        assert_eq!(workspace.tab_of_pane(target), Some(1));
        assert_eq!(workspace.pane_groups.groups()[0].panes, [first, second]);
    });
}
