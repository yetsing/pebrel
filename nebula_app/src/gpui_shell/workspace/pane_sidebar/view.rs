use super::*;
use crate::i18n::Message;
use gpui::{ObjectFit, StyledImage as _, img};

const HEADER_HEIGHT: f32 = 32.0;
const SESSION_HEIGHT: f32 = 74.0;
const GAP: f32 = 3.0;

struct TabPreview(SharedString);
impl Render for TabPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .child(self.0.clone())
    }
}

impl NebulaWorkspace {
    pub(in crate::gpui_shell::workspace) fn render_pane_sidebar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let language = crate::gpui_shell::config::ui_language(cx);
        let supported = actions::supports_project(&shell_launch::configured_local_launch(cx));
        let picking = self.pane_sidebar.picker.is_some();
        let hover_group: SharedString = "sidebar-tabs-header-hover".into();
        let rows = self
            .sidebar_rows()
            .into_iter()
            .map(|row| match row {
                Row::Project(directory) => self.render_project_header(directory, supported, cx),
                Row::Pane(id) => self.render_sidebar_pane(id, cx),
                Row::Tab(id) => self.render_sidebar_document(id, cx),
            })
            .collect::<Vec<_>>();
        v_flex()
            .w(px(self.sidebar_width))
            .h_full()
            .flex_shrink_0()
            .bg(crate::gpui_shell::theme::tab_bar_background(cx))
            .px_2()
            .pb_2()
            .gap_1()
            .child(
                h_flex()
                    .id("sidebar-tabs-toggle")
                    .debug_selector(|| "sidebar-tabs-toggle".into())
                    .group(hover_group.clone())
                    .h(px(HEADER_HEIGHT))
                    .w_full()
                    .items_center()
                    .child(
                        Button::new("sidebar-projects-toggle")
                            .ghost()
                            .xsmall()
                            .icon(if self.tabs_section_collapsed {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            })
                            .label(language.text(Message::WorkspaceSidebarProjects))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_tabs_section(cx))),
                    )
                    .child(div().flex_1())
                    .child(
                        super::super::sidebar::sidebar_new_tab_control(
                            hover_group,
                            cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.add_terminal(window, cx);
                            }),
                        )
                        .debug_selector(|| "sidebar-new-tab".into())
                        .rounded_md()
                        .hover(|style| style.bg(cx.theme().list_hover))
                        .tooltip(|window, cx| {
                            Tooltip::new(
                                crate::gpui_shell::config::ui_language(cx)
                                    .text(Message::ChromeNewTerminalCtrlShiftT),
                            )
                            .build(window, cx)
                        })
                        .child(Icon::new(IconName::Plus).small()),
                    )
                    .child(
                        Button::new("sidebar-shell-menu")
                            .ghost()
                            .xsmall()
                            .icon(IconName::EllipsisVertical)
                            .tooltip(language.text(Message::ChromeNewTerminalCtrlK))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_shell_palette(window, cx)
                            })),
                    ),
            )
            .child(
                Button::new("sidebar-open-project")
                    .debug_selector(|| "sidebar-open-project".into())
                    .ghost()
                    .small()
                    .w_full()
                    .icon(IconName::FolderOpen)
                    .label(if picking {
                        language.text(Message::CommonLoading)
                    } else {
                        language.text(Message::WorkspaceSidebarOpenProject)
                    })
                    .disabled(picking || !supported)
                    .tooltip(if supported {
                        language.text(Message::WorkspaceSidebarOpenProject)
                    } else {
                        language.text(Message::WorkspaceSidebarLocalShellRequired)
                    })
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_sidebar_project(window, cx)),
                    ),
            )
            .child(if self.tabs_section_collapsed {
                div().flex_1().into_any_element()
            } else {
                div()
                    .id("project-sidebar-frame")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .id("project-sidebar-list")
                            .debug_selector(|| "project-sidebar-list".into())
                            .size_full()
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.pane_sidebar.scroll)
                            .pr(px(14.0))
                            .gap(px(GAP))
                            .children(rows),
                    )
                    .child(div().absolute().top_0().right_0().bottom_0().w(px(14.0)).child(
                        gpui_component::scroll::Scrollbar::vertical(&self.pane_sidebar.scroll),
                    ))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_sidebar_pane(&self, id: u64, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(index) = self.tab_of_pane(id) else { return div().into_any_element() };
        let WorkspaceTab::Terminal { panes, focused, .. } = &self.tabs[index] else {
            return div().into_any_element();
        };
        let Some(pane) = panes.iter().find(|pane| pane.id == id) else {
            return div().into_any_element();
        };
        let theme = cx.theme();
        let group = self.pane_groups.groups().iter().find(|group| group.panes.contains(&id));
        let grouped = group.is_some();
        let last_in_group = group.is_some_and(|group| group.panes.last() == Some(&id));
        let language = crate::gpui_shell::config::ui_language(cx);
        let active = !self.settings_open && self.active == index && *focused == id;
        let title = self.pane_title(&pane.view, cx, theme.is_dark());
        let name: SharedString = pane
            .custom_name
            .clone()
            .or_else(|| (panes.len() == 1).then(|| self.meta(index).custom_name).flatten())
            .map(Into::into)
            .unwrap_or(title.text);
        let automatic_name = name.clone();
        let tooltip: SharedString = pane.view.read(cx).tab_tooltip(&name).into();
        let activity = super::super::sidebar::resting_activity(
            pane.view.read(cx).sidebar_activity(),
            active,
            panes.len() == 1 && self.meta(index).has_bell,
        );
        let icon = title
            .logo
            .map(|image| {
                img(image).size(px(17.0)).object_fit(ObjectFit::Contain).into_any_element()
            })
            .unwrap_or_else(|| {
                div()
                    .size(px(17.0))
                    .font_family(crate::font_install::SYMBOL_FONT_FAMILY)
                    .child(title.glyph.unwrap_or("\u{f489}"))
                    .into_any_element()
            });
        let status = self.render_sidebar_activity(activity, active, cx);
        let rename = self
            .pane_rename
            .as_ref()
            .filter(|edit| edit.pane_id == id)
            .map(|edit| edit.input.clone());
        let row = v_flex()
            .id(("sidebar-pane", id as usize))
            .debug_selector(move || format!("sidebar-pane-{id}"))
            .role(gpui::accesskit::Role::ListBoxOption)
            .aria_label(name.to_string())
            .focusable()
            .tab_stop(true)
            .relative()
            .w_full()
            .h(px(SESSION_HEIGHT))
            .flex_shrink_0()
            .pl(px(if grouped { 44.0 } else { 18.0 }))
            .pr_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .overflow_hidden()
            .text_size(px(13.0))
            .text_color(if active { theme.sidebar_accent_foreground } else { theme.foreground })
            .when(active, |row| row.bg(theme.sidebar_accent))
            .hover(|row| row.bg(theme.list_hover))
            .active(|row| row.bg(theme.list_active))
            .focus_visible(|row| row.border_color(theme.primary))
            .tooltip(move |window, cx| tab_presentation::tooltip(tooltip.clone(), window, cx))
            .on_click(
                cx.listener(move |this, _, window, cx| this.activate_sidebar_pane(id, window, cx)),
            )
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    this.activate_sidebar_pane(id, window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.close_sidebar_pane(id, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.activate_sidebar_pane(id, window, cx);
                    if let Some(index) = this.tab_of_pane(id) {
                        this.open_tab_context_menu(index, event.position, window, cx);
                    }
                }),
            )
            .on_double_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.begin_pane_rename(id, automatic_name.clone(), window, cx);
            }))
            .can_drop(|value, _, _| value.is::<windowing::CrossWindowTabDrag>())
            .on_drop(cx.listener(
                move |this, payload: &windowing::CrossWindowTabDrag, window, cx| {
                    cx.stop_propagation();
                    this.drop_sidebar_tab(payload, id, window, cx);
                },
            ))
            .when(active, |row| {
                row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(5.0))
                        .bottom(px(5.0))
                        .w(px(3.0))
                        .rounded_full()
                        .bg(theme.primary),
                )
            })
            .when(grouped, |row| {
                row.child(
                    div()
                        .absolute()
                        .left(px(28.0))
                        .top(px(-GAP))
                        .w(px(1.0))
                        .h(px(if last_in_group { 17.0 + GAP } else { SESSION_HEIGHT + GAP }))
                        .bg(theme.border),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(28.0))
                        .top(px(17.0))
                        .w(px(12.0))
                        .h(px(1.0))
                        .bg(theme.border),
                )
            })
            .child(
                h_flex()
                    .h(px(25.0))
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(icon)
                    .child(if let Some(input) = rename {
                        Input::new(&input).small().w_full().into_any_element()
                    } else {
                        div().flex_1().min_w_0().truncate().child(name.clone()).into_any_element()
                    })
                    .child(div().flex_shrink_0().child(status))
                    .when_some(self.cross_window_drag_payload(index, cx), |row, payload| {
                        row.child(
                            div()
                                .id(("sidebar-tab-grip", id as usize))
                                .debug_selector(move || format!("sidebar-tab-grip-{id}"))
                                .size(px(20.0))
                                .flex_shrink_0()
                                .cursor_grab()
                                .tooltip(move |window, cx| {
                                    Tooltip::new(
                                        crate::gpui_shell::config::ui_language(cx)
                                            .text(Message::WorkspaceSidebarDragTab),
                                    )
                                    .build(window, cx)
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, event, _, cx| {
                                        cx.stop_propagation();
                                        this.begin_sidebar_tab_drag(id, event, cx);
                                    }),
                                )
                                .on_drag(payload, move |_, _, _, cx| {
                                    cx.new(|_| TabPreview(name.clone()))
                                })
                                .child(
                                    Icon::new(IconName::EllipsisVertical)
                                        .xsmall()
                                        .text_color(theme.muted_foreground),
                                ),
                        )
                    })
                    .child(
                        Button::new(("sidebar-pane-close", id as usize))
                            .debug_selector(move || format!("sidebar-pane-close-{id}"))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip(language.text(Message::WorkspacePaneClose))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.close_sidebar_pane(id, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .debug_selector(move || format!("sidebar-pane-preview-{id}"))
                    .pl(px(25.0))
                    .w_full()
                    .h(px(32.0))
                    .overflow_hidden()
                    .line_clamp(2)
                    .text_ellipsis()
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(theme.muted_foreground)
                    .child(
                        pane.view.read(cx).submitted_preview().map(str::to_owned).unwrap_or_else(
                            || language.text(Message::WorkspaceSidebarNewSession).to_owned(),
                        ),
                    ),
            );
        row.into_any_element()
    }

    fn render_sidebar_activity(
        &self,
        activity: SidebarActivity,
        active: bool,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme();
        match activity {
            SidebarActivity::Running => {
                let (track, head) = crate::gpui_shell::theme::sidebar_spinner_colors(cx, active);
                Self::spinner(self.spinner_phase, track, head, cx.weak_entity()).into_any_element()
            },
            SidebarActivity::Done => {
                div().size(px(6.0)).rounded_full().bg(theme.primary).into_any_element()
            },
            SidebarActivity::WaitingInput | SidebarActivity::Attention => Self::waiting_hand(
                crate::font_install::SYMBOL_FONT_FAMILY.into(),
                13.0,
                if activity == SidebarActivity::Attention { theme.warning } else { theme.primary },
            )
            .into_any_element(),
            SidebarActivity::Idle => div().into_any_element(),
            other => Icon::new(match other {
                SidebarActivity::Paused => IconName::Pause,
                SidebarActivity::Completed => IconName::Check,
                SidebarActivity::CommandFailed => IconName::TriangleAlert,
                _ => IconName::CircleX,
            })
            .xsmall()
            .text_color(match other {
                SidebarActivity::Completed => theme.success,
                SidebarActivity::Paused => theme.warning,
                _ => theme.danger,
            })
            .into_any_element(),
        }
    }

    fn render_sidebar_document(
        &self,
        id: crate::runtime_api::tabs::TabId,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(index) = self.tab_meta.iter().position(|meta| meta.runtime_id == id) else {
            return div().into_any_element();
        };
        let presentation = self.tab_presentation(index, cx, cx.theme().is_dark());
        let active = !self.settings_open && self.active == index;
        let close = id.clone();
        let rename = id.clone();
        let keyboard = id.clone();
        let context = id.clone();
        let middle = id.clone();
        h_flex()
            .id(SharedString::from(format!("sidebar-document-{}", id.0)))
            .debug_selector({
                let id = id.clone();
                move || format!("sidebar-document-{}", id.0)
            })
            .focusable()
            .tab_stop(true)
            .h(px(HEADER_HEIGHT))
            .flex_shrink_0()
            .w_full()
            .px_2()
            .gap_2()
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(gpui::transparent_black())
            .when(active, |row| row.bg(cx.theme().sidebar_accent))
            .hover(|row| row.bg(cx.theme().list_hover))
            .active(|row| row.bg(cx.theme().list_active))
            .focus_visible(|row| row.border_color(cx.theme().primary))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    this.activate_sidebar_document(&keyboard, window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    if let Some(index) =
                        this.tab_meta.iter().position(|meta| meta.runtime_id == middle)
                    {
                        this.request_close_tab(index, window, cx);
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.activate_sidebar_document(&context, window, cx);
                    if let Some(index) =
                        this.tab_meta.iter().position(|meta| meta.runtime_id == context)
                    {
                        this.open_tab_context_menu(index, event.position, window, cx);
                    }
                }),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.activate_sidebar_document(&id, window, cx);
            }))
            .on_double_click(cx.listener(move |this, _, window, cx| {
                if let Some(index) = this.tab_meta.iter().position(|meta| meta.runtime_id == rename)
                {
                    this.begin_rename(index, window, cx);
                }
            }))
            .child(
                Icon::new(if presentation.is_settings {
                    IconName::Settings
                } else {
                    IconName::File
                })
                .small(),
            )
            .child(if let Some(input) = presentation.renaming {
                Input::new(&input).small().w_full().into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.0))
                    .child(presentation.title)
                    .into_any_element()
            })
            .child(
                Button::new(SharedString::from(format!("sidebar-document-close-{}", close.0)))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        if let Some(index) =
                            this.tab_meta.iter().position(|meta| meta.runtime_id == close)
                        {
                            this.request_close_tab(index, window, cx);
                        }
                    })),
            )
            .into_any_element()
    }
}
