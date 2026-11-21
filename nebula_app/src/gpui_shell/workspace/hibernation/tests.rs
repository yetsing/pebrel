use super::super::*;
use super::*;
use crate::platform::project_directory::ProjectDirectory;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn click_control(selector: &str, window: &mut Window, cx: &mut App) {
    let position = window.debug_bounds_for_selector(selector).unwrap().center();
    for event in [
        gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers: Default::default(),
        }),
        gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        }),
        gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        }),
    ] {
        window.dispatch_event(event, cx);
        let _ = window.draw(cx);
    }
}

#[test]
#[ignore = "requires Windows desktop, PEBREL_HIBERNATION_QA_DIR and isolated PEBREL_CONFIG_DIR"]
fn sleeping_tab_stays_reachable_and_wakes_with_its_frozen_project() {
    let output = PathBuf::from(std::env::var_os("PEBREL_HIBERNATION_QA_DIR").unwrap());
    assert_eq!(
        std::env::var_os("PEBREL_CONFIG_DIR").map(PathBuf::from),
        Some(output.join("config"))
    );
    assert!(!output.join("verified.json").exists(), "use a fresh fixture directory");
    let directory = output.join("project");
    let cwd = output.join("cwd");
    for path in [&directory, &cwd, &output.join("config")] {
        std::fs::create_dir_all(path).unwrap();
    }
    let project = ProjectDirectory::new(&directory).unwrap();
    let checked = Arc::new(AtomicBool::new(false));
    let after_run = checked.clone();
    gpui_platform::application().with_assets(crate::gpui_shell::assets::NebulaAssets).run(
        move |cx| {
            let hub = crate::runtime_api::RuntimeHub::new();
            gpui_component::init(cx);
            crate::gpui_shell::register_bundled_fonts(cx);
            cx.set_global(crate::gpui_shell::config::Settings::load(
                nebula_settings::ThemeName::Nord,
            ));
            crate::gpui_shell::theme::apply_chrome_theme(cx);
            crate::gpui_shell::math_view::register(cx);
            crate::gpui_shell::file_editor::init(cx);
            super::super::init(cx);
            windowing::initialize(cx, hub.clone());
            cx.set_reduce_motion(true);
            let mut workspace = None;
            let handle = cx
                .open_window(
                    gpui::WindowOptions {
                        window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::new(
                            gpui::point(px(80.0), px(80.0)),
                            gpui::size(px(1100.0), px(700.0)),
                        ))),
                        focus: false,
                        show: true,
                        ..Default::default()
                    },
                    |window, cx| {
                        let view = cx.new(|cx| {
                            let mut view = NebulaWorkspace::new(
                                window,
                                None,
                                None,
                                1,
                                hub,
                                windowing::WorkspaceStartup::Empty,
                                windowing::WindowRole::Regular,
                                cx,
                            );
                            view.tabs_position = nebula_settings::TabsPositionName::Sidebar;
                            view.sidebar_width = 280.0;
                            view.sidebar_collapsed = false;
                            view.insert_tab_at(
                                0,
                                WorkspaceTab::Sleeping(Box::new(SleepingTab {
                                    title: "sleeping fixture".into(),
                                    cwd: cwd.clone(),
                                    focus: cx.focus_handle(),
                                    launch: LaunchSession::Shell {
                                        name: "missing".into(),
                                        program: "pebrel-test-missing-shell-executable".into(),
                                        args: vec![],
                                    },
                                    grid: (80, 24),
                                    history: None,
                                    pane_name: Some("original pane".into()),
                                    pane_name_history: vec![None],
                                    project: Some(pane_groups::model::PaneProject::fixed(
                                        project.clone(),
                                    )),
                                })),
                                TabMeta::default(),
                            );
                            view
                        });
                        workspace = Some(view.clone());
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
                .unwrap();
            let workspace = workspace.unwrap();
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_millis(250)).await;
                cx.update_window(handle.into(), |_, window, cx| {
                    workspace.update(cx, |workspace, cx| {
                        let id = workspace.meta(0).runtime_id.clone();
                        assert_eq!(workspace.sidebar_rows(), [pane_sidebar::Row::Tab(id)]);
                        workspace.wake_tab(0, window, cx);
                        assert!(
                            matches!(workspace.tabs[0], WorkspaceTab::Sleeping(_)),
                            "failed launch must keep the sleeping tab"
                        );
                        let WorkspaceTab::Sleeping(saved) = &mut workspace.tabs[0] else {
                            unreachable!()
                        };
                        saved.launch = LaunchSession::Shell {
                            name: "CMD".into(),
                            program: "cmd.exe".into(),
                            args: vec!["/Q".into()],
                        };
                        cx.notify();
                    });
                    let _ = window.draw(cx);
                    let id = workspace
                        .read_with(cx, |workspace, _| workspace.meta(0).runtime_id.clone());
                    click_control(&format!("sidebar-document-{}", id.0), window, cx);
                    workspace.update(cx, |workspace, cx| {
                        let pane = workspace.tabs[0]
                            .primary_terminal_pane()
                            .expect("row click must wake the tab");
                        assert_eq!(pane.project.as_ref().unwrap().directory(), Some(&project));
                        assert_eq!(pane.custom_name.as_deref(), Some("original pane"));
                        assert_eq!(pane.view.read(cx).cwd, cwd.to_string_lossy());
                        assert!(
                            pane.view.read(cx).session.is_some(),
                            "wake must create a real PTY"
                        );
                        assert_eq!(workspace.pane_groups.groups()[0].panes, [pane.id]);
                        assert_eq!(
                            workspace.sidebar_rows(),
                            [
                                pane_sidebar::Row::Project(project.clone()),
                                pane_sidebar::Row::Pane(pane.id)
                            ]
                        );
                        workspace.close_tab(0, window, cx);
                        assert!(workspace.pane_groups.groups().is_empty());
                        assert!(workspace.sidebar_rows().is_empty());
                    });
                    std::fs::write(
                        output.join("verified.json"),
                        serde_json::to_vec(&serde_json::json!({
                            "failed_launch_retained": true, "row_click_woke": true,
                            "project_restored": true, "closed_cleanly": true,
                        }))
                        .unwrap(),
                    )
                    .unwrap();
                    checked.store(true, Ordering::SeqCst);
                    window.remove_window();
                })
                .unwrap();
                cx.update(|cx| cx.quit());
            })
            .detach();
        },
    );
    assert!(after_run.load(Ordering::SeqCst), "native sleep regression did not complete");
}
