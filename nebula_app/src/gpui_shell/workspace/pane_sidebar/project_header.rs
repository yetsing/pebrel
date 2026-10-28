use super::*;
use crate::i18n::Message;

impl NebulaWorkspace {
    pub(super) fn render_project_header(
        &self,
        directory: ProjectDirectory,
        supported: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let language = crate::gpui_shell::config::ui_language(cx);
        let theme = cx.theme();
        let Some(group) =
            self.pane_groups.groups().iter().find(|group| group.directory == directory)
        else {
            return div().into_any_element();
        };
        let collapsed = group.collapsed;
        let pinned = group.pinned;
        let selected = !self.settings_open
            && self.active_terminal_pane_id().is_some_and(|id| group.panes.contains(&id));
        let path: SharedString = directory.path().to_string_lossy().into_owned().into();
        let name: SharedString = directory
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string())
            .into();
        let fold_label = language.text(if collapsed {
            Message::WorkspaceSidebarExpandProject
        } else {
            Message::WorkspaceSidebarCollapseProject
        });
        let fold = directory.clone();
        let keyboard = directory.clone();
        let pin = directory.clone();
        let branch = self.pane_sidebar.branches.get(&directory).map(str::to_owned);
        v_flex()
            .id(SharedString::from(format!("sidebar-project-{path}")))
            .debug_selector({
                let path = path.clone();
                move || format!("sidebar-project-{path}")
            })
            .relative()
            .h(px(60.0))
            .pt(px(8.0))
            .flex_shrink_0()
            .w_full()
            .text_color(theme.foreground)
            .child(
                h_flex()
                    .h(px(30.0))
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child(
                        h_flex()
                            .id(SharedString::from(format!("sidebar-project-fold-{path}")))
                            .debug_selector({
                                let path = path.clone();
                                move || format!("sidebar-project-fold-{path}")
                            })
                            .role(gpui::accesskit::Role::Button)
                            .aria_expanded(!collapsed)
                            .aria_label(format!("{fold_label} {name}"))
                            .focusable()
                            .tab_stop(true)
                            .h_full()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_1()
                            .px_1()
                            .rounded_md()
                            .border_1()
                            .border_color(gpui::transparent_black())
                            .cursor_pointer()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(collapsed && selected, |row| row.bg(theme.sidebar_accent))
                            .hover(|row| row.bg(theme.list_hover))
                            .active(|row| row.bg(theme.list_active))
                            .focus_visible(|row| row.border_color(theme.primary))
                            .tooltip(move |window, cx| {
                                tab_presentation::tooltip(path.clone(), window, cx)
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.toggle_sidebar_project(&fold, cx);
                            }))
                            .on_key_down(cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if matches!(
                                        event.keystroke.key.as_str(),
                                        "enter" | "space" | "left" | "right"
                                    ) {
                                        cx.stop_propagation();
                                        let should_toggle = match event.keystroke.key.as_str() {
                                            "left" => !collapsed,
                                            "right" => collapsed,
                                            _ => true,
                                        };
                                        if should_toggle {
                                            this.toggle_sidebar_project(&keyboard, cx);
                                        }
                                    }
                                },
                            ))
                            .child(
                                Icon::new(if collapsed {
                                    IconName::ChevronRight
                                } else {
                                    IconName::ChevronDown
                                })
                                .xsmall()
                                .flex_shrink_0(),
                            )
                            .child(Icon::new(IconName::Folder).small().flex_shrink_0())
                            .child(div().flex_1().min_w_0().truncate().child(name)),
                    )
                    .child(
                        Button::new(SharedString::from(format!(
                            "sidebar-project-pin-{}",
                            directory.path().display()
                        )))
                        .debug_selector({
                            let path = directory.path().to_owned();
                            move || format!("sidebar-project-pin-{}", path.display())
                        })
                        .ghost()
                        .xsmall()
                        .toggled(pinned)
                        .icon(Icon::new(Icon::empty()).path(crate::gpui_shell::assets::nav::PIN))
                        .tooltip(language.text(if pinned {
                            Message::WorkspaceSidebarUnpinProject
                        } else {
                            Message::WorkspaceSidebarPinProject
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.pin_sidebar_project(&pin, cx);
                        })),
                    )
                    .child(
                        Button::new(SharedString::from(format!(
                            "sidebar-project-add-{}",
                            directory.path().display()
                        )))
                        .debug_selector({
                            let path = directory.path().to_owned();
                            move || format!("sidebar-project-add-{}", path.display())
                        })
                        .ghost()
                        .xsmall()
                        .icon(IconName::Plus)
                        .disabled(!supported)
                        .tooltip(language.text(if supported {
                            Message::WorkspaceSidebarNewSessionInProject
                        } else {
                            Message::WorkspaceSidebarLocalShellRequired
                        }))
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.add_project_session(directory.clone(), window, cx);
                            },
                        )),
                    ),
            )
            .child(
                h_flex().pl(px(40.0)).pr_2().h(px(18.0)).w_full().gap_1().items_center().when_some(
                    branch,
                    |row, branch| {
                        let tooltip: SharedString = language
                            .format(Message::WorkspaceSidebarProjectBranch, &[("branch", &branch)])
                            .into();
                        row.child(
                            div()
                                .font_family(crate::font_install::SYMBOL_FONT_FAMILY)
                                .text_size(px(11.0))
                                .text_color(theme.muted_foreground)
                                .child("\u{ea68}"),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "sidebar-project-branch-{}",
                                    group.directory.path().display()
                                )))
                                .debug_selector({
                                    let directory = group.directory.clone();
                                    move || {
                                        format!(
                                            "sidebar-project-branch-{}",
                                            directory.path().display()
                                        )
                                    }
                                })
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(11.0))
                                .text_color(theme.muted_foreground)
                                .tooltip(move |window, cx| {
                                    tab_presentation::tooltip(tooltip.clone(), window, cx)
                                })
                                .child(branch),
                        )
                    },
                ),
            )
            .into_any_element()
    }
}
