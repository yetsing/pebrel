//! Opt-in desktop fixture; all paths and terminal sessions belong to its QA directory.

use super::super::pane_groups::model::PaneProject;
use super::*;
use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

fn click_control(selector: &str, window: &mut Window, cx: &mut App) {
    let position = window.debug_bounds_for_selector(selector).unwrap().center();
    window.dispatch_event(
        gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers: Default::default(),
        }),
        cx,
    );
    let _ = window.draw(cx);
    window.dispatch_event(
        gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        }),
        cx,
    );
    let _ = window.draw(cx);
    window.dispatch_event(
        gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        }),
        cx,
    );
    let _ = window.draw(cx);
}

#[test]
#[ignore = "requires Windows desktop, PEBREL_SIDEBAR_QA_DIR and isolated PEBREL_CONFIG_DIR"]
fn native_project_sidebar() {
    let output = std::path::PathBuf::from(std::env::var_os("PEBREL_SIDEBAR_QA_DIR").unwrap());
    assert_eq!(
        std::env::var_os("PEBREL_CONFIG_DIR").map(std::path::PathBuf::from),
        Some(output.join("config"))
    );
    assert!(!output.join("ready.json").exists(), "use a fresh fixture directory");
    let a = output.join("Project Alpha with a long directory name");
    let b = output.join("项目 Beta");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let theme = if std::env::var("PEBREL_SIDEBAR_QA_THEME").as_deref() == Ok("light") {
        nebula_settings::ThemeName::Paper
    } else {
        nebula_settings::ThemeName::Nord
    };
    let narrow = std::env::var("PEBREL_SIDEBAR_QA_NARROW").as_deref() == Ok("1");
    std::fs::create_dir_all(output.join("config")).unwrap();
    std::fs::write(output.join("config/pebrel_settings.txt"), format!(
        "theme={}\nfollow_system_theme=false\nshell=cmd\ntabs_position=sidebar\nsidebar_w={}\nlanguage={}\nnew_tab_position=end\n",
        if theme == nebula_settings::ThemeName::Paper { "Paper" } else { "Nord" },
        if narrow { 220 } else { 300 },
        if theme == nebula_settings::ThemeName::Paper { "en-US" } else { "zh-CN" },
    )).unwrap();
    let checked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let after_run = checked.clone();
    gpui_platform::application().with_assets(crate::gpui_shell::assets::NebulaAssets).run(move |cx| {
        let hub = crate::runtime_api::RuntimeHub::new();
        gpui_component::init(cx);
        crate::gpui_shell::register_bundled_fonts(cx);
        cx.set_global(crate::gpui_shell::config::Settings::load(theme));
        crate::gpui_shell::theme::apply_chrome_theme(cx);
        crate::gpui_shell::math_view::register(cx);
        crate::gpui_shell::file_editor::init(cx);
        super::super::init(cx);
        windowing::initialize(cx, hub.clone());
        let mut result = None;
        let handle = cx.open_window(gpui::WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::new(
                gpui::point(px(80.0), px(80.0)), gpui::size(px(1100.0), px(720.0)),
            ))), focus: false, show: true, ..Default::default()
        }, |window, cx| {
            let workspace = cx.new(|cx| NebulaWorkspace::new(window, None, None, 1, hub,
                windowing::WorkspaceStartup::Empty, windowing::WindowRole::Regular, cx));
            workspace.update(cx, |this, cx| {
                this.tabs_position = nebula_settings::TabsPositionName::Sidebar;
                this.sidebar_width = if narrow { 220.0 } else { 300.0 };
                this.sidebar_collapsed = false;
                this.tabs_section_collapsed = false;
                let mut ids = Vec::new();
                for path in [&a, &b] {
                    let id = this.add_terminal_for_project(crate::session::LaunchSession::Shell {
                        name: "Command Prompt".into(), program: "C:\\Windows\\System32\\cmd.exe".into(),
                        args: vec!["/Q".into()],
                    }, Some(path.clone()), None,
                    Some(PaneProject::fixed(ProjectDirectory::new(path).unwrap())), window, cx);
                    ids.push(id);
                    if ids.len() == 1 {
                        ids.push(this.split_focused(SplitDirection::LeftRight, window, cx).unwrap());
                    }
                }
                if let WorkspaceTab::Terminal { panes, .. } = &mut this.tabs[0] {
                    panes[0].custom_name = Some("Build and inspect the sidebar".into());
                    panes[1].custom_name = Some("检查长名称与分栏焦点".into());
                }
                if std::env::var("PEBREL_SIDEBAR_QA_PREVIEW").as_deref() == Ok("1") {
                    for (index, pane) in this.tabs.iter().filter_map(|tab| {
                        if let WorkspaceTab::Terminal { panes, .. } = tab { Some(panes) } else { None }
                    }).flatten().enumerate() {
                        pane.view.update(cx, |view, cx| {
                            let token = format!("native-preview-{index}");
                            for (name, value) in [
                                ("pebrel_shell", token.clone()),
                                ("pebrel_input", format!("{token}\n检查最近提交的多行消息与窄侧栏换行。\nInspect multiline messages, long names and pane navigation without shifting the close button.")),
                            ] {
                                view.remote_files_test_event(nebula_terminal::event::Event::UserVar { name: name.into(), value }, cx);
                            }
                            assert!(view.submitted_preview().is_some());
                        });
                    }
                }
                this.activate_sidebar_pane(ids[0], window, cx);
                result = Some((workspace.clone(), ids));
                cx.notify();
            });
            cx.new(|cx| Root::new(workspace, window, cx))
        }).unwrap();
        let (workspace, ids) = result.unwrap();
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_millis(1000)).await;
            cx.update_window(handle.into(), |_, window, cx| {
                let _ = window.draw(cx);
                let RawWindowHandle::Win32(raw) = window.window_handle().unwrap().as_raw() else { panic!("Windows required") };
                let controls = ids.iter().flat_map(|id| [format!("sidebar-pane-{id}"), format!("sidebar-pane-close-{id}"), format!("sidebar-tab-grip-{id}")])
                    .filter_map(|selector| window.debug_bounds_for_selector(&selector).map(|bounds| {
                        let scale = window.scale_factor();
                        (selector, [f32::from(bounds.center().x) * scale, f32::from(bounds.center().y) * scale])
                    })).collect::<std::collections::BTreeMap<_, _>>();
                std::fs::write(output.join("ready.json"), serde_json::to_vec(&serde_json::json!({"hwnd": raw.hwnd.get(), "panes": ids, "controls": controls})).unwrap()).unwrap();
            }).unwrap();
            let mut stage = 0;
            for _ in 0..150 {
                let _ = cx.update_window(handle.into(), |_, window, cx| {
                    if stage == 0 && output.join("select-pane").exists() {
                        click_control(&format!("sidebar-pane-{}", ids[1]), window, cx);
                        workspace.read_with(cx, |workspace, _| assert_eq!(workspace.active_terminal_pane_id(), Some(ids[1])));
                        stage = 1;
                    }
                    if stage == 1 && output.join("close-pane").exists() {
                        click_control(&format!("sidebar-pane-close-{}", ids[1]), window, cx);
                        workspace.read_with(cx, |workspace, _| {
                            assert!(workspace.tab_of_pane(ids[1]).is_none());
                            assert!(workspace.tab_of_pane(ids[0]).is_some());
                            assert_eq!(workspace.tabs.len(), 2);
                        });
                        stage = 2;
                        checked.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    workspace.update(cx, |workspace, cx| {
                        std::fs::write(output.join("state.json"), serde_json::to_vec(&serde_json::json!({
                            "stage": stage,
                            "active": workspace.active_terminal_pane_id(), "tabs": workspace.tabs.len(),
                            "panes": workspace.sidebar_rows().iter().filter_map(|row| if let Row::Pane(id) = row { Some(*id) } else { None }).collect::<Vec<_>>()
                        })).unwrap()).unwrap();
                        cx.notify();
                    });
                    let _ = window.draw(cx);
                });
                if output.join("capture-complete").exists() { break; }
                cx.background_executor().timer(Duration::from_millis(200)).await;
            }
            let _ = cx.update_window(handle.into(), |_, window, cx| {
                workspace.update(cx, |workspace, cx| {
                    while !workspace.tabs.is_empty() { workspace.close_tab(0, window, cx); }
                });
                window.remove_window();
            });
            cx.update(|cx| cx.quit());
        }).detach();
    });
    assert!(
        after_run.load(std::sync::atomic::Ordering::SeqCst),
        "native pane selection and close were not completed"
    );
}
